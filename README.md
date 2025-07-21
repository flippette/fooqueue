# fooqueue

a simple, lock-free FIFO queue.

## example

```rust
use std::thread;

use fooqueue::Queue;

// no need to be `mut`!
let queue = Queue::new();

queue.push(1);
queue.push(2);
queue.push(3);

assert_eq!(queue.len(), 3);
assert_eq!(queue.pop(), Some(3));
assert_eq!(queue.pop(), Some(2));
assert_eq!(queue.pop(), Some(1));
assert!(queue.is_empty());

// can be used from multiple threads!
thread::scope(|s| {
  for _ in 0..6 {
    let queue = &queue;
    s.spawn(move || {
      for i in 0..1024 {
        queue.push(i);
      }
    });
  }

  for _ in 0..2 {
    let queue = &queue;
    s.spawn(move || {
      for _ in 0..512 {
        while queue.pop().is_none() {}
      }
    });
  }
});

assert_eq!(queue.len(), 6 * 1024 - 2 * 512);
```
