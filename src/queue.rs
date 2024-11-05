use super::Job;
use parking_lot::{Condvar, Mutex};
use std::{
    collections::VecDeque,
    sync::atomic::{AtomicUsize, Ordering},
};

#[derive(Default)]
pub struct JobQueue {
    inner: Mutex<QueueInner>,
    not_empty: Condvar,
    waiters: AtomicUsize,
}

impl Drop for JobQueue {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl JobQueue {
    pub fn new() -> Self {
        Self {
            inner: Default::default(),
            not_empty: Condvar::new(),
            waiters: AtomicUsize::new(0),
        }
    }

    /// Enqueue job and notify sleeping thread
    pub fn add(&self, job: Job) {
        let mut lock = self.inner.lock();

        if lock.shutdown {
            return;
        }

        lock.queue.push_back(job);
        self.not_empty.notify_one();
    }

    pub fn waiters(&self) -> usize {
        self.waiters.load(Ordering::Acquire)
    }

    /// Set finish flag and wake up sleeping threads
    pub fn shutdown(&self) {
        self.inner.lock().shutdown = true;
        self.not_empty.notify_all();
    }

    /// Get next task from the queue. If the queue is empty, then thread sleeps until
    /// adding new elements.
    ///
    /// Return `None` if the queue will not give out elements no more.
    pub fn get_job(&self) -> Option<Job> {
        let mut lock = self.inner.lock();

        let mut waiting = false;

        while lock.queue.is_empty() {
            // Probably, the thread woken up because it's time to return
            if lock.shutdown {
                return None;
            };

            // If there are no elements, then thread is going to sleep
            self.waiters.fetch_add(1, Ordering::Release);
            waiting = true;
            self.not_empty.wait(&mut lock);
        }

        // Now we can assert that there are definitely elements in the queue
        assert!(!lock.queue.is_empty());

        if waiting {
            self.waiters.fetch_sub(1, Ordering::Release);
        }

        let job = lock
            .queue
            .pop_front()
            .expect("there must be prepared job(s)");

        Some(job)
    }
}

/// The internal queue and shutdown flag are separated into a different structure because they
/// will be locked by the mutex at the same time; otherwise, workers might sleep after
/// the shutdown flag has been set.
#[derive(Default)]
struct QueueInner {
    queue: VecDeque<Job>,
    shutdown: bool,
}
