//! the underlying queue.

#[cfg(all(feature = "nightly", feature = "alloc"))]
use alloc::alloc::Global;
#[cfg(feature = "nightly")]
use core::alloc::{AllocError, Allocator};
use core::hint;
use core::ptr::{self, NonNull};
use core::sync::atomic::Ordering::*;

#[cfg(all(feature = "allocator-api2", feature = "alloc"))]
use allocator_api2::alloc::Global;
#[cfg(feature = "allocator-api2")]
use allocator_api2::alloc::{AllocError, Allocator};
use portable_atomic::AtomicPtr;

use crate::consumer::Consumer;
use crate::node::Node;
use crate::producer::Producer;

/// a simple, lock-free FIFO queue.
#[cfg(not(feature = "alloc"))]
pub struct Queue<T, A>
where
  A: Allocator,
{
  alloc: A,
  head: AtomicPtr<Node<T>>,
}

/// a simple, lock-free FIFO queue.
#[cfg(feature = "alloc")]
pub struct Queue<T, A = Global>
where
  A: Allocator,
{
  alloc: A,
  head: AtomicPtr<Node<T>>,
}

// SAFETY:
//   - we only pass `T` and not `&T` => `T: Send`
//   - we manipulate the queue with atomics => `Sync`
#[rustfmt::skip]
unsafe impl<T, A> Send for Queue<T, A>
where T: Send, A: Allocator {}
#[rustfmt::skip]
unsafe impl<T, A> Sync for Queue<T, A>
where T: Send, A: Allocator {}

impl<T> Queue<T, Global> {
  /// create a new queue.
  pub const fn new() -> Self {
    Self::new_in(Global)
  }
}

// public APIs
impl<T, A> Queue<T, A>
where
  A: Allocator,
{
  /// create a new queue with a given allocator.
  pub const fn new_in(alloc: A) -> Self {
    let head = AtomicPtr::new(ptr::null_mut());
    Self { alloc, head }
  }

  /// split a queue into a [`Producer`] and a [`Consumer`].
  pub const fn split(&mut self) -> (Producer<'_, T, A>, Consumer<'_, T, A>) {
    let queue = &*self;
    (Producer { queue }, Consumer { queue })
  }

  /// check if the queue is empty.
  pub fn is_empty(&self) -> bool {
    self.head.load(Acquire).is_null()
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
    let new_head = self.make_node(elem)?.as_ptr();
    let old_head = *self.head.get_mut();
    // SAFETY:
    //   - `new_head` came from `self.make_node`, so it's valid.
    //   - we take a `&mut self`, so there's no data race here.
    unsafe { (&raw mut (*new_head).next).write(old_head) };
    self.head = AtomicPtr::new(new_head);
    Ok(())
  }

  /// pop an element from the front.
  pub fn pop(&mut self) -> Option<T> {
    let old_head = *self.head.get_mut();
    if old_head.is_null() {
      return None;
    }
    let new_head = unsafe { (*old_head).next };
    self.head = AtomicPtr::new(new_head);
    // SAFETY:
    //   - `old_head` came from our allocator.
    //   - it is valid.
    //   - we take `&mut self`, so there's no data race here.
    Some(unsafe { self.consume_node(old_head) })
  }
}

impl<T> Default for Queue<T, Global> {
  fn default() -> Self {
    Self::new()
  }
}

impl<T, A> Drop for Queue<T, A>
where
  A: Allocator,
{
  fn drop(&mut self) {
    while self.pop().is_some() {}
  }
}

/// private APIs.
impl<T, A> Queue<T, A>
where
  A: Allocator,
{
  /// try to push an element to the front atomically, returning an error on
  /// allocation failure.
  pub(crate) fn try_push_atomic(&self, elem: T) -> Result<(), (T, AllocError)> {
    let new_head = self.make_node(elem)?.as_ptr();

    loop {
      let old_head = self.head.load(Acquire);
      // SAFETY: `new_head` is a valid pointer to a `Node<T>`.
      unsafe { (&raw mut (*new_head).next).write(old_head) }
      if self
        .head
        .compare_exchange_weak(old_head, new_head, Release, Acquire)
        .is_ok()
      {
        return Ok(());
      }
      hint::spin_loop();
    }
  }

  /// pop an element off the front atomically.
  ///
  /// # safety
  ///
  /// this method must not be called from multiple threads.
  pub(crate) unsafe fn pop_atomic(&self) -> Option<T> {
    loop {
      let old_head = self.head.load(Acquire);
      if old_head.is_null() {
        return None;
      }

      // SAFETY: `old_head` came from our allocator, and is not null.
      let new_head = unsafe { (*old_head).next };
      if self
        .head
        .compare_exchange_weak(old_head, new_head, Release, Acquire)
        .is_ok()
      {
        // SAFETY:
        //   - `old_head` came from our allocator.
        //   - it is valid.
        //   - no one else can refer to it.
        return Some(unsafe { self.consume_node(old_head) });
      }

      hint::spin_loop();
    }
  }

  /// allocate a new node with our allocator and initialize it.
  fn make_node(&self, data: T) -> Result<NonNull<Node<T>>, (T, AllocError)> {
    match self
      .alloc
      .allocate(Node::<T>::LAYOUT)
      .map(NonNull::cast::<Node<T>>)
    {
      // SAFETY: `ptr` is a successful, valid allocation.
      Ok(nn) => unsafe {
        let ptr = nn.as_ptr();
        (&raw mut (*ptr).next).write(ptr::null_mut());
        (&raw mut (*ptr).data).write(data);
        Ok(nn)
      },
      Err(err) => Err((data, err)),
    }
  }

  /// deallocate the node, and return the previously contained data.
  ///
  /// # safety
  ///
  /// `node` must be a valid pointer, and must have come from our allocator.
  /// this method also must not be called twice on the same pointer.
  unsafe fn consume_node(&self, node: *mut Node<T>) -> T {
    unsafe {
      let data = (&raw mut (*node).data).read();
      self
        .alloc
        .deallocate(NonNull::new_unchecked(node).cast(), Node::<T>::LAYOUT);
      data
    }
  }
}
