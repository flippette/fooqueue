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
use portable_atomic::{AtomicPtr, AtomicUsize};

#[cfg(not(feature = "alloc"))]
/// a lock-free FIFO queue.
pub struct Queue<T, A>
where
  A: Allocator,
{
  alloc: A,
  head: AtomicPtr<Node<T>>,
  len: AtomicUsize,
}

#[cfg(feature = "alloc")]
/// a lock-free FIFO queue.
pub struct Queue<T, A = Global>
where
  A: Allocator,
{
  alloc: A,
  head: AtomicPtr<Node<T>>,
  len: AtomicUsize,
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
    const { Self::new_in(Global) }
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
      len: AtomicUsize::new(0),
    }
  }

  /// get the length of the queue.
  pub fn len(&self) -> usize {
    self.len.load(Acquire)
  }

  /// is the queue empty?
  pub fn is_empty(&self) -> bool {
    self.len() == 0
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

  #[test]
  fn threads() {
    let queue = Queue::new_in(Global);

    queue.push(1);
    queue.push(2);
    queue.push(3);

    assert_eq!(queue.len(), 3);
    assert_eq!(queue.pop(), Some(3));
    assert_eq!(queue.pop(), Some(2));
    assert_eq!(queue.pop(), Some(1));
    assert!(queue.is_empty());

    thread::scope(|s| {
      for _ in 0..14 {
        let queue = &queue;
        s.spawn(move || {
          for i in 0..4096 {
            queue.push(i);
          }
        });
      }

      for _ in 0..2 {
        let queue = &queue;
        s.spawn(move || {
          for _ in 0..1024 {
            while queue.pop().is_none() {}
          }
        });
      }
    });

    assert_eq!(queue.len(), 14 * 4096 - 2 * 1024);
  }
}
