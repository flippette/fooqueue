#![doc = include_str!("../README.md")]
#![cfg_attr(not(test), no_std)]
#![cfg_attr(
  feature = "nightly",
  feature(allocator_api),
  expect(unstable_features)
)]

#[cfg(feature = "alloc")]
extern crate alloc;

mod consumer;
mod node;
mod producer;
mod queue;

pub use consumer::Consumer;
pub use producer::Producer;
pub use queue::Queue;

#[cfg(test)]
mod tests {
  use core::cell::Cell;
  #[cfg(feature = "nightly")]
  use std::alloc::Global;
  use std::thread;

  #[cfg(feature = "allocator-api2")]
  use allocator_api2::alloc::Global;

  use super::Queue;

  #[test]
  fn push_pop() {
    let mut queue = Queue::new_in(Global);

    queue.push(1);
    queue.push(2);
    queue.push(3);
    assert_eq!(queue.pop(), Some(3));
    assert_eq!(queue.pop(), Some(2));
    queue.push(4);
    assert_eq!(queue.pop(), Some(4));
    assert_eq!(queue.pop(), Some(1));
    assert_eq!(queue.pop(), None);
  }

  #[test]
  fn drops() {
    let mut queue = Queue::new_in(Global);
    let drops = Cell::new(0);

    struct DetectDrop<'a>(&'a Cell<i32>);
    impl Drop for DetectDrop<'_> {
      fn drop(&mut self) {
        self.0.update(|n| n + 1);
      }
    }

    queue.push(DetectDrop(&drops));
    queue.push(DetectDrop(&drops));
    queue.push(DetectDrop(&drops));
    queue.push(DetectDrop(&drops));
    queue.push(DetectDrop(&drops));
    drop(queue);

    assert_eq!(drops.get(), 5);
  }

  #[cfg(not(miri))]
  #[test]
  fn threads() {
    threads_impl::<8, 4096, 1024>();
  }

  #[cfg(miri)]
  #[test]
  fn threads() {
    threads_impl::<4, 128, 64>();
  }

  fn threads_impl<
    const PUSH_THREADS: usize,
    const PUSH_COUNT: usize,
    const POP_COUNT: usize,
  >() {
    let mut queue = Queue::new_in(Global);
    let (tx, mut rx) = queue.split();

    thread::scope(|s| {
      for _ in 0..PUSH_THREADS {
        let mut tx = tx.clone();
        s.spawn(move || {
          for i in 0..PUSH_COUNT {
            tx.push(i);
          }
        });
      }

      s.spawn(move || {
        for _ in 0..POP_COUNT {
          while rx.pop().is_none() {}
        }
      });
    });

    let mut len = 0;
    while let Some(_) = queue.pop() {
      len += 1;
    }
    assert_eq!(len, PUSH_THREADS * PUSH_COUNT - POP_COUNT);
  }
}
