#![doc = include_str!("../README.md")]
#![cfg_attr(not(test), no_std)]
#![cfg_attr(
  feature = "nightly",
  feature(allocator_api),
  expect(unstable_features)
)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(all(feature = "nightly", feature = "alloc"))]
use alloc::alloc::Global;
#[cfg(feature = "nightly")]
use core::alloc::{AllocError, Allocator, Layout};
use core::ptr;
use core::sync::atomic::Ordering::*;

#[cfg(all(feature = "allocator-api2", feature = "alloc"))]
use allocator_api2::alloc::Global;
#[cfg(feature = "allocator-api2")]
use allocator_api2::alloc::{AllocError, Allocator, Layout};
use portable_atomic::AtomicPtr;

/// a lock-free FIFO queue.
#[cfg(not(feature = "alloc"))]
pub struct Queue<T, A>
where
  A: Allocator,
{
  alloc: A,
  head: AtomicPtr<Node<T>>,
}

/// a lock-free FIFO queue.
#[cfg(feature = "alloc")]
pub struct Queue<T, A = Global>
where
  A: Allocator,
{
  alloc: A,
  head: AtomicPtr<Node<T>>,
}

/// a queue node.
struct Node<T> {
  next: AtomicPtr<Self>,
  data: T,
}

// SAFETY: we send instances of `T` across threads, so `T` needs only be `Send`.
// also, we manipulate the queue with atomics, so we can implement `Sync`.
unsafe impl<T, A> Send for Queue<T, A>
where
  T: Send,
  A: Allocator,
{
}

// SAFETY: see above.
unsafe impl<T, A> Sync for Queue<T, A>
where
  T: Send,
  A: Allocator,
{
}

#[cfg(feature = "alloc")]
impl<T> Queue<T, Global> {
  /// create a new queue.
  pub const fn new() -> Self {
    Self::new_in(Global)
  }
}

impl<T, A> Queue<T, A>
where
  A: Allocator,
{
  /// create an empty queue in a given allocator.
  pub const fn new_in(alloc: A) -> Self {
    Self {
      alloc,
      head: AtomicPtr::new(ptr::null_mut()),
    }
  }

  /// is the queue empty?
  pub fn is_empty(&self) -> bool {
    self.head.load(Acquire).is_null()
  }

  /// push an item to the front of the queue.
  ///
  /// for the non-panicking variant, see [`Queue::try_push`].
  pub fn push(&self, elem: T) {
    self
      .try_push(elem)
      .map_err(|(_, err)| err)
      .expect("failed to allocate space for push")
  }

  /// try to push an item to the front of the queue, returning the passed
  /// element on allocation failure.
  pub fn try_push(&self, elem: T) -> Result<(), (T, AllocError)> {
    todo!()
  }

  /// pop an item from the front of the queue.
  pub fn pop(&self) -> Option<T> {
    todo!()
  }

  /// get the [`Layout`] of a queue node.
  const fn node_layout() -> Layout {
    Layout::new::<Node<T>>()
  }
}

impl<T, A> Default for Queue<T, A>
where
  A: Allocator + Default,
{
  /// create an empty queue.
  fn default() -> Self {
    Self::new_in(A::default())
  }
}

impl<T, A> Drop for Queue<T, A>
where
  A: Allocator,
{
  /// drop all remaining elements in the queue.
  fn drop(&mut self) {
    while let Some(_) = self.pop() {}
  }
}

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
    let queue = Queue::new_in(Global);

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
    let queue = Queue::new_in(Global);
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
    threads_impl::<8, 4096, 8, 1024>();
  }

  #[cfg(miri)]
  #[test]
  fn threads() {
    threads_impl::<4, 128, 4, 64>();
  }

  fn threads_impl<
    const PUSH_THREADS: usize,
    const PUSH_COUNT: usize,
    const POP_THREADS: usize,
    const POP_COUNT: usize,
  >() {
    let queue = Queue::new_in(Global);

    thread::scope(|s| {
      for _ in 0..PUSH_THREADS {
        let queue = &queue;
        s.spawn(move || {
          for i in 0..PUSH_COUNT {
            queue.push(i);
          }
        });
      }

      for _ in 0..POP_THREADS {
        let queue = &queue;
        s.spawn(move || {
          for _ in 0..POP_COUNT {
            while queue.pop().is_none() {}
          }
        });
      }
    });

    let mut len = 0;
    while let Some(_) = queue.pop() {
      len += 1;
    }
    assert_eq!(len, PUSH_THREADS * PUSH_COUNT - POP_THREADS * POP_COUNT);
  }
}
