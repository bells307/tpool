mod error;
mod inner;
mod queue;
mod worker;

pub use error::JoinError;
use inner::ThreadPoolInner;
use std::{num::NonZeroUsize, sync::Arc};

#[cfg(test)]
mod tests;

/// Job for worker
pub type Job = Box<dyn Fn() + Send>;

/// "Lazy" thread pool for executing tasks. A new thread is created only if the current
/// number of threads is insufficient.
#[derive(Clone)]
pub struct ThreadPool(Arc<ThreadPoolInner>);

impl ThreadPool {
    /// Create new `ThreadPool`
    pub fn new(max_threads: NonZeroUsize) -> Self {
        Self(ThreadPoolInner::new(max_threads))
    }

    /// Spawn new job for thread pool
    pub fn spawn(&self, job: impl Fn() + Send + 'static) {
        ThreadPoolInner::spawn(&self.0, job)
    }

    /// Wait for thread pool to complete all jobs
    pub fn join(self) -> Result<(), JoinError> {
        self.0.join()
    }
}
