#![no_std]
#![cfg_attr(all(feature = "alloc", not(feature = "std")), feature(alloc))]

//! A library that provides a way to logically own objects, whether or not
//! heap allocation is available.

#[cfg(feature = "std")]
extern crate std;
#[cfg(all(feature = "alloc", not(feature = "std")))]
extern crate alloc;

mod object;
mod slice;

pub use object::Managed;
pub use slice::ManagedSlice;
