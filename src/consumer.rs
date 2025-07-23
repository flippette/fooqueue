//! a consumer pops items from its [`Queue`].

#[cfg(all(feature = "nightly", feature = "alloc"))]
use alloc::alloc::Global;
#[cfg(feature = "nightly")]
use core::alloc::Allocator;

#[cfg(feature = "allocator-api2")]
use allocator_api2::alloc::Allocator;
#[cfg(all(feature = "allocator-api2", feature = "alloc"))]
use allocator_api2::alloc::Global;

use crate::queue::Queue;

/// a consumer pops items from its [`Queue`].
#[cfg(not(feature = "alloc"))]
pub struct Consumer<'q, T, A>
where
  A: Allocator,
{
  pub(crate) queue: &'q Queue<T, A>,
}

/// a consumer pops items from its [`Queue`].
#[cfg(feature = "alloc")]
pub struct Consumer<'q, T, A = Global>
where
  A: Allocator,
{
  pub(crate) queue: &'q Queue<T, A>,
}

impl<'q, T, A> Consumer<'q, T, A>
where
  A: Allocator,
{
  /// get the underlying queue.
  pub const fn queue(&self) -> &Queue<T, A> {
    self.queue
  }

  /// pop an element from the front.
  pub fn pop(&mut self) -> Option<T> {
    // SAFETY:
    //   - we take `&mut self`.
    //   - this struct is not cloneable.
    //   => we're the only one who can call this at any given moment.
    unsafe { self.queue.pop_atomic() }
  }
}
