use super::Job;
use parking_lot::{Condvar, Mutex};
use std::{
    collections::VecDeque,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

#[derive(Default)]
pub struct JobQueue {
    queue: Mutex<VecDeque<Job>>,
    not_empty: Condvar,
    finished: AtomicBool,
    waiters: AtomicUsize,
}

impl JobQueue {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    /// Enqueue job and notify sleeping thread
    pub fn add(&self, job: Job) {
        if self.finished.load(Ordering::Relaxed) {
            return;
        }

        self.queue.lock().push_back(job);
        self.not_empty.notify_one();
    }

    pub fn waiters(&self) -> usize {
        self.waiters.load(Ordering::Acquire)
    }

    /// Set finish flag and wake up sleeping threads
    pub fn finish_ntf(&self) {
        self.finished.store(true, Ordering::Relaxed);
        self.not_empty.notify_all();
    }

    /// Get next task from the queue. If the queue is empty, then thread sleeps until
    /// adding new elements.
    ///
    /// Return `None` if the queue will not give out elements no more.
    pub fn get_blocked(&self) -> Option<Job> {
        if self.finished.load(Ordering::Relaxed) {
            return None;
        }

        let mut lock = self.queue.lock();

        let mut waiting = false;

        while lock.is_empty() {
            // If there are no elements, then thread is going to sleep
            self.waiters.fetch_add(1, Ordering::Release);
            waiting = true;
            self.not_empty.wait(&mut lock);

            // Probably, the thread woken up because it's time to return
            if self.finished.load(Ordering::Relaxed) {
                return None;
            }
        }

        // Now we can assert that there are definitely elements in the queue
        assert!(!lock.is_empty());

        if waiting {
            self.waiters.fetch_sub(1, Ordering::Release);
        }

        Some(lock.pop_front().expect("there must be prepared job(s)"))
    }
}
