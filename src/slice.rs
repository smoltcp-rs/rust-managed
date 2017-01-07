use core::ops::{Deref, DerefMut};
use core::fmt;

#[cfg(feature = "use_std")]
use std::vec::Vec;
#[cfg(feature = "use_collections")]
use collections::vec::Vec;

/// A managed slice.
///
/// This enum can be used to represent exclusive access to slices of objects.
/// In Rust, exclusive access to an object is obtained by either owning the object,
/// or owning a mutable pointer to the object; hence, "managed".
///
/// The purpose of this enum is providing good ergonomics with `std` present while making
/// it possible to avoid having a heap at all (which of course means that `std` is not present).
/// To achieve this, the variants other than `Borrow` are only available when the corresponding
/// feature is opted in.
///
/// A function that requires a managed object should be generic over an `Into<ManagedSlice<'a, T>>`
/// argument; then, it will be possible to pass either a `Vec<T>`, or a `&'a mut [T]`
/// without any conversion at the call site.
///
/// See also [Managed][struct.Managed.html].
pub enum ManagedSlice<'a, T: 'a> {
    /// Borrowed variant.
    Borrowed(&'a mut [T]),
    /// Owned variant, only available with the `use_std` or `use_collections` feature enabled.
    #[cfg(any(feature = "use_std", feature = "use_collections"))]
    Owned(Vec<T>)
}

impl<'a, T: 'a> fmt::Debug for ManagedSlice<'a, T>
        where T: fmt::Debug {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &ManagedSlice::Borrowed(ref x) => write!(f, "Borrowed({:?})", x),
            #[cfg(any(feature = "use_std", feature = "use_collections"))]
            &ManagedSlice::Owned(ref x)    => write!(f, "Owned({:?})", x)
        }

    }
}

impl<'a, T: 'a> From<&'a mut [T]> for ManagedSlice<'a, T> {
    fn from(value: &'a mut [T]) -> Self {
        ManagedSlice::Borrowed(value)
    }
}

#[cfg(any(feature = "use_std", feature = "use_collections"))]
impl<T: 'static> From<Vec<T>> for ManagedSlice<'static, T> {
    fn from(value: Vec<T>) -> Self {
        ManagedSlice::Owned(value)
    }
}

impl<'a, T: 'a> Deref for ManagedSlice<'a, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        match self {
            &ManagedSlice::Borrowed(ref value) => value,
            #[cfg(any(feature = "use_std", feature = "use_collections"))]
            &ManagedSlice::Owned(ref value) => value
        }
    }
}

impl<'a, T: 'a> DerefMut for ManagedSlice<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            &mut ManagedSlice::Borrowed(ref mut value) => value,
            #[cfg(any(feature = "use_std", feature = "use_collections"))]
            &mut ManagedSlice::Owned(ref mut value) => value
        }
    }
}
