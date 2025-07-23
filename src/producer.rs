//! a producer pushes items to its [`Queue`].

#[cfg(all(feature = "nightly", feature = "alloc"))]
use alloc::alloc::Global;
#[cfg(feature = "nightly")]
use core::alloc::{AllocError, Allocator};

#[cfg(all(feature = "allocator-api2", feature = "alloc"))]
use allocator_api2::alloc::Global;
#[cfg(feature = "allocator-api2")]
use allocator_api2::alloc::{AllocError, Allocator};

use crate::queue::Queue;

/// a producer pushes items to its [`Queue`].
#[cfg(not(feature = "alloc"))]
#[derive(Clone)]
pub struct Producer<'q, T, A>
where
  A: Allocator,
{
  pub(crate) queue: &'q Queue<T, A>,
}

/// a producer pushes items to its [`Queue`].
#[cfg(feature = "alloc")]
#[derive(Clone)]
pub struct Producer<'q, T, A = Global>
where
  A: Allocator,
{
  pub(crate) queue: &'q Queue<T, A>,
}

impl<'q, T, A> Producer<'q, T, A>
where
  A: Allocator,
{
  /// get the underlying queue.
  pub const fn queue(&self) -> &Queue<T, A> {
    self.queue
  }

  /// push an element to the front.
  ///
  /// for the non-panicking variant, see [`Queue::try_push`].
  pub fn push(&mut self, elem: T) {
    self
      .try_push(elem)
      .map_err(|(_, err)| err)
      .expect("push to queue failed");
  }

  /// try to push an element to the front, returning an error on allocation
  /// failure.
  pub fn try_push(&mut self, elem: T) -> Result<(), (T, AllocError)> {
    self.queue.try_push_atomic(elem)
  }
}
