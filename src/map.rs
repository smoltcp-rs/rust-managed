use core::mem;
use core::fmt;
use core::borrow::Borrow;

#[cfg(feature = "std")]
use std::collections::BTreeMap;
#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::btree_map::BTreeMap;

/// A managed map.
///
/// This enum can be used to represent exclusive access to maps.
/// In Rust, exclusive access to an object is obtained by either owning the object,
/// or owning a mutable pointer to the object; hence, "managed".
///
/// The purpose of this enum is providing good ergonomics with `std` present while making
/// it possible to avoid having a heap at all (which of course means that `std` is not present).
/// To achieve this, the variants other than `Borrow` are only available when the corresponding
/// feature is opted in.
///
/// Unlike [Managed](enum.Managed.html) and [ManagedSlice](enum.ManagedSlice.html),
/// the managed map is implemented using a B-tree map when allocation is available,
/// and a sorted slice of key-value pairs when it is not. Thus, algorithmic complexity
/// of operations on it depends on the kind of map.
///
/// A function that requires a managed object should be generic over an `Into<ManagedMap<'a, T>>`
/// argument; then, it will be possible to pass either a `Vec<T>`, or a `&'a mut [T]`
/// without any conversion at the call site.
///
/// See also [Managed](enum.Managed.html).
pub enum ManagedMap<'a, K: 'a, V: 'a> {
    /// Borrowed variant.
    Borrowed(&'a mut [Option<(K, V)>]),
    /// Owned variant, only available with the `std` or `alloc` feature enabled.
    #[cfg(any(feature = "std", feature = "alloc"))]
    Owned(BTreeMap<K, V>)
}

impl<'a, K: 'a, V: 'a> fmt::Debug for ManagedMap<'a, K, V>
        where K: fmt::Debug, V: fmt::Debug {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &ManagedMap::Borrowed(ref x) => write!(f, "Borrowed({:?})", x),
            #[cfg(any(feature = "std", feature = "alloc"))]
            &ManagedMap::Owned(ref x)    => write!(f, "Owned({:?})", x)
        }
    }
}

impl<'a, K: 'a, V: 'a> From<&'a mut [Option<(K, V)>]> for ManagedMap<'a, K, V> {
    fn from(value: &'a mut [Option<(K, V)>]) -> Self {
        ManagedMap::Borrowed(value)
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl<'a, K: 'a, V: 'a> From<BTreeMap<K, V>> for ManagedMap<'a, K, V> {
    fn from(value: BTreeMap<K, V>) -> Self {
        ManagedMap::Owned(value)
    }
}

/// Like `Option`, but with `Some` values sorting first.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum RevOption<T> {
    Some(T),
    None
}

impl<T> From<Option<T>> for RevOption<T> {
    fn from(other: Option<T>) -> Self {
        match other {
            Some(x) => RevOption::Some(x),
            None => RevOption::None
        }
    }
}

impl<T> Into<Option<T>> for RevOption<T> {
    fn into(self) -> Option<T> {
        match self {
            RevOption::Some(x) => Some(x),
            RevOption::None => None
        }
    }
}

fn binary_search_by_key<K, V, Q>(slice: &[Option<(K, V)>], key: &Q) -> Result<usize, usize>
    where K: Ord + Borrow<Q>, Q: Ord + ?Sized
{
    slice.binary_search_by_key(&RevOption::Some(key), |entry| {
        entry.as_ref().map(|&(ref key, _)| key.borrow()).into()
    })
}

fn pair_by_key<'a, K, Q, V>(slice: &'a [Option<(K, V)>], key: &Q) ->
                           Result<&'a (K, V), usize>
    where K: Ord + Borrow<Q>, Q: Ord + ?Sized
{
    binary_search_by_key(slice, key).map(move |idx| slice[idx].as_ref().unwrap())
}

fn pair_mut_by_key<'a, K, Q, V>(slice: &'a mut [Option<(K, V)>], key: &Q) ->
                               Result<&'a mut (K, V), usize>
    where K: Ord + Borrow<Q>, Q: Ord + ?Sized
{
    binary_search_by_key(slice, key).map(move |idx| slice[idx].as_mut().unwrap())
}

impl<'a, K: Ord + 'a, V: 'a> ManagedMap<'a, K, V> {
    pub fn clear(&mut self) {
        match self {
            &mut ManagedMap::Borrowed(ref mut pairs) => {
                for item in pairs.iter_mut() {
                    *item = None
                }
            },
            #[cfg(any(feature = "std", feature = "alloc"))]
            &mut ManagedMap::Owned(ref mut map) => map.clear()
        }
    }

    pub fn get<Q>(&self, key: &Q) -> Option<&V>
        where K: Borrow<Q>, Q: Ord + ?Sized
    {
        match self {
            &ManagedMap::Borrowed(ref pairs) => {
                match pair_by_key(pairs, key.borrow()) {
                    Ok(&(_, ref value)) => Some(value),
                    Err(_) => None
                }
            },
            #[cfg(any(feature = "std", feature = "alloc"))]
            &ManagedMap::Owned(ref map) => map.get(key)
        }
    }

    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
        where K: Borrow<Q>, Q: Ord + ?Sized
    {
        match self {
            &mut ManagedMap::Borrowed(ref mut pairs) => {
                match pair_mut_by_key(pairs, key.borrow()) {
                    Ok(&mut (_, ref mut value)) => Some(value),
                    Err(_) => None
                }
            },
            #[cfg(any(feature = "std", feature = "alloc"))]
            &mut ManagedMap::Owned(ref mut map) => map.get_mut(key)
        }
    }

    pub fn insert(&mut self, key: K, new_value: V) -> Result<Option<V>, (K, V)> {
        match self {
            &mut ManagedMap::Borrowed(ref mut pairs) => {
                match binary_search_by_key(pairs, &key) {
                    Err(_) if pairs[pairs.len() - 1].is_some() =>
                        Err((key, new_value)), // full
                    Err(idx) => {
                        let rotate_by = pairs.len() - 1;
                        pairs[idx..].rotate(rotate_by);
                        pairs[idx] = Some((key, new_value));
                        Ok(None)
                    }
                    Ok(idx) => {
                        let mut swap_pair = Some((key, new_value));
                        mem::swap(&mut pairs[idx], &mut swap_pair);
                        let (_key, value) = swap_pair.unwrap();
                        Ok(Some(value))
                    }
                }
            },
            #[cfg(any(feature = "std", feature = "alloc"))]
            &mut ManagedMap::Owned(ref mut map) => Ok(map.insert(key, new_value))
        }
    }

    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
        where K: Borrow<Q>, Q: Ord + ?Sized
    {
        match self {
            &mut ManagedMap::Borrowed(ref mut pairs) => {
                match binary_search_by_key(pairs, key) {
                    Ok(idx) => {
                        let (_key, value) = pairs[idx].take().unwrap();
                        pairs[idx..].rotate(1);
                        Some(value)
                    }
                    Err(_) => None
                }
            },
            #[cfg(any(feature = "std", feature = "alloc"))]
            &mut ManagedMap::Owned(ref mut map) => map.remove(key)
        }
    }
}

