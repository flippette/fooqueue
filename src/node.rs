//! nodes in a [`crate::queue::Queue`].

#[cfg(feature = "nightly")]
use core::alloc::Layout;

#[cfg(feature = "allocator-api2")]
use allocator_api2::alloc::Layout;

/// a queue node.
pub struct Node<T> {
  pub next: *mut Self,
  pub data: T,
}

impl<T> Node<T> {
  /// the memory layout of a node.
  pub const LAYOUT: Layout = Layout::new::<Self>();
}
