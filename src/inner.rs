use super::{queue::JobQueue, JoinError};
use drop_panic::drop_panic;
use parking_lot::Mutex;
use std::{
    collections::HashMap,
    num::NonZeroUsize,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread::{self, JoinHandle, ThreadId},
};

pub(super) struct ThreadPoolInner {
    job_queue: JobQueue,
    threads: Mutex<Option<HashMap<ThreadId, JoinHandle<()>>>>,
    thread_count: AtomicUsize,
    max_threads: usize,
}

impl ThreadPoolInner {
    pub fn new(max_threads: NonZeroUsize) -> Arc<Self> {
        Arc::new(Self {
            job_queue: JobQueue::new(),
            threads: Mutex::new(None),
            thread_count: AtomicUsize::new(0),
            max_threads: max_threads.get(),
        })
    }

    /// Enqueue job
    pub fn spawn(this: &Arc<Self>, job: impl Fn() + Send + 'static) {
        // Lazy creation of new threads: only if there are no threads ready to take on
        // the task, do we create a new one
        if this.thread_count.load(Ordering::Acquire) < this.max_threads
            && this.job_queue.waiters() == 0
        {
            Arc::clone(this).create_worker();
        }

        this.job_queue.add(Box::new(job));
    }

    pub fn join(&self) -> Result<(), JoinError> {
        // Notify queue about finish and ask it to wake sleeping threads
        self.job_queue.finish_ntf();

        let mut panicked = 0;

        let Some(threads) = self.threads.lock().take() else {
            return Ok(());
        };

        threads.into_values().for_each(|jh| {
            jh.join().inspect_err(|_| panicked += 1).ok();
        });

        if panicked == 0 {
            Ok(())
        } else {
            Err(JoinError::new(panicked))
        }
    }

    /// Create a thread for executing tasks
    fn create_worker(self: Arc<Self>) {
        let this = Arc::clone(&self);

        let jh = thread::Builder::new()
            .spawn(move || {
                // In case of thread panic, this object will call the recovery function
                drop_panic! {
                    Arc::clone(&this).panic_handler()
                };

                // Start working cycle
                Arc::clone(&this).worker_routine()
            })
            .expect("spawn worker thread");

        let id = jh.thread().id();

        let mut lock = self.threads.lock();

        match *lock {
            Some(ref mut threads) => match threads.get_mut(&id) {
                Some(_) => panic!("thread with id {:?} already created", id),
                None => {
                    threads.insert(id, jh);
                    self.thread_count.fetch_add(1, Ordering::Release);
                }
            },
            None => *lock = Some(HashMap::from([(id, jh)])),
        }
    }

    /// Working thread panic handling
    fn panic_handler(&self) {
        let id = thread::current().id();

        if let Some(threads) = self.threads.lock().as_mut() {
            threads.remove(&id);
        };
    }

    /// Thread working cycle
    fn worker_routine(&self) {
        loop {
            let Some(job) = self.job_queue.get_blocked() else {
                return;
            };

            job();
        }
    }
}
