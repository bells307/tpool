# tpool
A "lazy" thread pool for executing tasks. This thread pool only creates new threads when there are no idle threads available to handle incoming jobs. It aims to be efficient by minimizing the number of threads and only spawning new ones as needed.

### Example

```rust
use tpool::ThreadPool;
use std::sync::mpsc

fn main() {
    let thread_count = 8;

    let tp = ThreadPool::new(thread_count.try_into().unwrap());

    let (tx, rx) = mpsc::channel();

    for _ in 0..thread_count {
        let tx = tx.clone();
        tp.spawn(move || {
            tx.send(1).unwrap();
        });
    }

    assert_eq!(rx.iter().take(thread_count).sum::<usize>(), thread_count);
}
```
