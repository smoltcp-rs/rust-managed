#![no_std]
#![cfg_attr(all(feature = "alloc", not(feature = "std")), feature(alloc))]
#![cfg_attr(feature = "map", feature(slice_rotate))]
#![cfg_attr(feature = "map", feature(collections_range))]

//! A library that provides a way to logically own objects, whether or not
//! heap allocation is available.

#[cfg(feature = "std")]
extern crate std;
#[cfg(all(feature = "alloc", not(feature = "std")))]
extern crate alloc;

mod object;
mod slice;
#[cfg(feature = "map")]
mod map;

pub use object::Managed;
pub use slice::ManagedSlice;
#[cfg(feature = "map")]
pub use map::{ManagedMap,
              Iter as ManagedMapIter,
              IterMut as ManagedMapIterMut};
