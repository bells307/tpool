use super::{queue::JobQueue, JoinError};
use crate::worker::{Worker, WorkerId};
use parking_lot::Mutex;
use std::{
    collections::HashMap,
    num::NonZeroUsize,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread::ThreadId,
};

pub(super) struct ThreadPoolInner {
    job_queue: JobQueue,
    workers: Mutex<Option<HashMap<ThreadId, Worker>>>,
    thread_count: AtomicUsize,
    max_threads: usize,
}

impl ThreadPoolInner {
    pub(super) fn new(max_threads: NonZeroUsize) -> Arc<Self> {
        Arc::new(Self {
            job_queue: JobQueue::new(),
            workers: Mutex::new(None),
            thread_count: AtomicUsize::new(0),
            max_threads: max_threads.get(),
        })
    }

    /// Enqueue job
    pub(super) fn spawn(this: &Arc<Self>, job: impl Fn() + Send + 'static) {
        // Lazy creation of new threads: only if there are no threads ready to take on
        // the task, do we create a new one
        if this.thread_count.load(Ordering::Acquire) < this.max_threads
            && this.job_queue.waiters() == 0
        {
            Self::create_worker(this);
        }

        this.job_queue.add(Box::new(job));
    }

    pub(super) fn join(&self) -> Result<(), JoinError> {
        // Notify queue about finish and ask it to wake sleeping threads
        self.job_queue.shutdown();

        let mut panicked = 0;

        let Some(threads) = self.workers.lock().take() else {
            return Ok(());
        };

        threads.into_values().for_each(|worker| {
            if !worker.join() {
                panicked += 1;
            };
        });

        if panicked == 0 {
            Ok(())
        } else {
            Err(JoinError::new(panicked))
        }
    }

    pub(super) fn remove_worker(&self, id: WorkerId) {
        if let Some(threads) = self.workers.lock().as_mut() {
            threads.remove(&id);
        };
    }

    pub(super) fn job_queue(&self) -> &JobQueue {
        &self.job_queue
    }

    /// Create a thread for executing tasks
    fn create_worker(this: &Arc<Self>) {
        let worker = Worker::new(Arc::clone(this));

        let id = worker.id();

        let mut lock = this.workers.lock();

        match *lock {
            Some(ref mut threads) => match threads.get_mut(&id) {
                Some(_) => panic!("worker with id {:?} already created", id),
                None => {
                    threads.insert(id, worker);
                    this.thread_count.fetch_add(1, Ordering::Release);
                }
            },
            None => *lock = Some(HashMap::from([(id, worker)])),
        }
    }
}
