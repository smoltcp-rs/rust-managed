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
                        let rotate_by = pairs.len() - idx - 1;
                        pairs[idx..].rotate_left(rotate_by);
                        assert!(pairs[idx].is_none(), "broken invariant");
                        pairs[idx] = Some((key, new_value));
                        Ok(None)
                    }
                    Ok(idx) => {
                        let mut swap_pair = Some((key, new_value));
                        mem::swap(&mut pairs[idx], &mut swap_pair);
                        let (_key, value) = swap_pair.expect("broken invariant");
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
                        let (_key, value) = pairs[idx].take().expect("broken invariant");
                        pairs[idx..].rotate_left(1);
                        Some(value)
                    }
                    Err(_) => None
                }
            },
            #[cfg(any(feature = "std", feature = "alloc"))]
            &mut ManagedMap::Owned(ref mut map) => map.remove(key)
        }
    }

    /// ManagedMap contains no elements?
    pub fn is_empty(&self) -> bool {
        match self {
            &ManagedMap::Borrowed(ref pairs) =>
                pairs.iter().all(|item| item.is_none()),
            #[cfg(any(feature = "std", feature = "alloc"))]
            &ManagedMap::Owned(ref map) =>
                map.is_empty()
        }
    }

    /// Returns the number of elements in the ManagedMap.
    pub fn len(&self) -> usize {
        match self {
            &ManagedMap::Borrowed(ref pairs) =>
                pairs.iter()
                .filter(|item| item.is_some())
                .count(),
            #[cfg(any(feature = "std", feature = "alloc"))]
            &ManagedMap::Owned(ref map) =>
                map.len()
        }
    }
}

// LCOV_EXCL_START
#[cfg(test)]
mod test {
    use super::ManagedMap;

    fn all_pairs_empty() -> [Option<(&'static str, u32)>; 4] {
        [None; 4]
    }

    fn one_pair_full() -> [Option<(&'static str, u32)>; 4] {
        [Some(("a", 1)), None, None, None]
    }

    fn all_pairs_full() -> [Option<(&'static str, u32)>; 4] {
        [Some(("a", 1)), Some(("b", 2)), Some(("c", 3)), Some(("d", 4))]
    }

    fn unwrap<'a, K, V>(map: &'a ManagedMap<'a, K, V>) -> &'a [Option<(K, V)>] {
        match map {
            &ManagedMap::Borrowed(ref map) => map,
            _ => unreachable!()
        }
    }

    #[test]
    fn test_clear() {
        let mut pairs = all_pairs_full();
        let mut map = ManagedMap::Borrowed(&mut pairs);
        map.clear();
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);
        assert_eq!(unwrap(&map), all_pairs_empty());
    }

    #[test]
    fn test_get_some() {
        let mut pairs = all_pairs_full();
        let map = ManagedMap::Borrowed(&mut pairs);
        assert_eq!(map.len(), 4);
        assert_eq!(map.get("a"), Some(&1));
        assert_eq!(map.get("b"), Some(&2));
        assert_eq!(map.get("c"), Some(&3));
        assert_eq!(map.get("d"), Some(&4));
    }

    #[test]
    fn test_get_none() {
        let mut pairs = one_pair_full();
        let map = ManagedMap::Borrowed(&mut pairs);
        assert_eq!(map.len(), 1);
        assert!(!map.is_empty());
        assert_eq!(map.get("q"), None);
    }

    #[test]
    fn test_get_mut_some() {
        let mut pairs = all_pairs_full();
        let mut map = ManagedMap::Borrowed(&mut pairs);
        assert_eq!(map.len(), 4);
        assert!(!map.is_empty());
        assert_eq!(map.get_mut("a"), Some(&mut 1));
        assert_eq!(map.get_mut("b"), Some(&mut 2));
        assert_eq!(map.get_mut("c"), Some(&mut 3));
        assert_eq!(map.get_mut("d"), Some(&mut 4));
    }

    #[test]
    fn test_get_mut_none() {
        let mut pairs = one_pair_full();
        let mut map = ManagedMap::Borrowed(&mut pairs);
        assert_eq!(map.get_mut("q"), None);
    }

    #[test]
    fn test_insert_empty() {
        let mut pairs = all_pairs_empty();
        let mut map = ManagedMap::Borrowed(&mut pairs);
        assert_eq!(map.len(), 0);
        assert!(map.is_empty());

        assert_eq!(map.insert("a", 1), Ok(None));
        assert_eq!(map.len(), 1);
        assert!(!map.is_empty());
        assert_eq!(unwrap(&map),       [Some(("a", 1)), None, None, None]);
    }

    #[test]
    fn test_insert_replace() {
        let mut pairs = all_pairs_empty();
        let mut map = ManagedMap::Borrowed(&mut pairs);
        assert_eq!(map.insert("a", 1), Ok(None));
        assert_eq!(map.insert("a", 2), Ok(Some(1)));
        assert_eq!(map.len(), 1);
        assert!(!map.is_empty());
        assert_eq!(unwrap(&map),       [Some(("a", 2)), None, None, None]);
    }

    #[test]
    fn test_insert_full() {
        let mut pairs = all_pairs_full();
        let mut map = ManagedMap::Borrowed(&mut pairs);
        assert_eq!(map.insert("q", 1), Err(("q", 1)));
        assert_eq!(map.len(), 4);
        assert_eq!(unwrap(&map),       all_pairs_full());
    }

    #[test]
    fn test_insert_one() {
        let mut pairs = one_pair_full();
        let mut map = ManagedMap::Borrowed(&mut pairs);
        assert_eq!(map.insert("b", 2), Ok(None));
        assert_eq!(unwrap(&map),       [Some(("a", 1)), Some(("b", 2)), None, None]);
    }

    #[test]
    fn test_insert_shift() {
        let mut pairs = one_pair_full();
        let mut map = ManagedMap::Borrowed(&mut pairs);
        assert_eq!(map.insert("c", 3), Ok(None));
        assert_eq!(map.insert("b", 2), Ok(None));
        assert_eq!(unwrap(&map),       [Some(("a", 1)), Some(("b", 2)), Some(("c", 3)), None]);
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut pairs = one_pair_full();
        let mut map = ManagedMap::Borrowed(&mut pairs);
        assert_eq!(map.remove("b"), None);
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_remove_one() {
        let mut pairs = all_pairs_full();
        let mut map = ManagedMap::Borrowed(&mut pairs);
        assert_eq!(map.remove("a"), Some(1));
        assert_eq!(map.len(), 3);
        assert_eq!(unwrap(&map),    [Some(("b", 2)), Some(("c", 3)), Some(("d", 4)), None]);
    }
}

