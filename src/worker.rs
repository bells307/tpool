use crate::inner::ThreadPoolInner;
use scopeguard::defer;
use std::{
    sync::Arc,
    thread::{self, JoinHandle, ThreadId},
};

pub(super) type WorkerId = ThreadId;

pub(super) struct Worker(JoinHandle<()>);

impl Worker {
    pub(super) fn new(inner: Arc<ThreadPoolInner>) -> Self {
        let jh = thread::spawn(|| Self::worker_loop(inner));
        Self(jh)
    }

    /// Wait for the worker to finish. If the thread panicked, it returns `false`
    pub(super) fn join(self) -> bool {
        self.0.join().is_ok()
    }

    pub(super) fn id(&self) -> WorkerId {
        self.0.thread().id()
    }

    fn worker_loop(inner: Arc<ThreadPoolInner>) {
        // Remove the thread from the pool. If the thread panicked during the task it was given to
        // execute, this function will also be called upon unwinding the stack, and the panic will
        // be handled correctly
        defer! {
            let id = thread::current().id();
            inner.remove_worker(id);
        };

        // Start working cycle
        loop {
            let Some(job) = inner.job_queue().get_job() else {
                break;
            };

            job();
        }
    }
}
