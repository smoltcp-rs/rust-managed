#![no_std]
#![cfg_attr(feature = "use_alloc", feature(alloc))]
#![cfg_attr(feature = "use_collections", feature(collections))]

#[cfg(feature = "use_std")]
extern crate std;
#[cfg(feature = "use_alloc")]
extern crate alloc;
#[cfg(feature = "use_collections")]
extern crate collections;

mod object;
mod slice;

pub use object::Managed;
pub use slice::ManagedSlice;
