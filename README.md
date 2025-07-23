# fooqueue

a simple, lock-free FIFO queue.

## example

```rust
use std::thread;

use fooqueue::Queue;

// no need to be `mut`!
let mut queue = Queue::new();

{
  let (mut tx, mut rx) = queue.split();

  tx.push(1);
  tx.push(2);
  tx.push(3);

  assert!(!tx.queue().is_empty());
  assert_eq!(rx.pop(), Some(3));
  assert_eq!(rx.pop(), Some(2));
  assert_eq!(rx.pop(), Some(1));
  assert!(tx.queue().is_empty());

  // can be used from multiple threads!
  thread::scope(|s| {
    for _ in 0..6 {
      // producers can be cloned cheaply
      let mut tx = tx.clone();
      s.spawn(move || {
        for i in 0..1024 {
          tx.push(i);
        }
      });
    }

    // only one consumer may exist
    s.spawn(move || {
      for i in 0..512 {
        rx.pop();
      }
    });
  });
}

// queue can be re-split after the tx-rx pair expires
let (_tx, _rx) = queue.split();
```
