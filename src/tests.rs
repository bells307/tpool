use crate::ThreadPool;
use std::{
    sync::mpsc,
    thread::{self},
    time::Duration,
};

#[test]
fn test_thread_pool1() {
    let thread_count = 2;

    let tp = ThreadPool::new(thread_count.try_into().unwrap());

    let (tx, rx) = mpsc::channel();

    for _ in 0..thread_count {
        let tx = tx.clone();
        tp.spawn(move || {
            tx.send(1).unwrap();
        });
    }

    assert_eq!(rx.iter().take(thread_count).sum::<usize>(), thread_count);

    tp.join().unwrap();
}

#[test]
fn test_thread_pool_panic() {
    let tp = ThreadPool::new(4.try_into().unwrap());

    tp.spawn(|| {
        thread::sleep(Duration::from_secs(1));
        panic!("boom");
    });

    thread::sleep(Duration::from_secs(5));
}
