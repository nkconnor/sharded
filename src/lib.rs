//! # sharded &emsp; ![Build] ![Crate]
//!
//! [Build]: https://github.com/nkconnor/sharded/workflows/build/badge.svg
//! [Crate]: https://img.shields.io/crates/v/sharded
//!
//! **Sharded provides safe, fast, and obvious concurrent collections in Rust**. This crate splits the
//! underlying collection into `N shards` each with its own lock.
//!
//! ## Features
//!
//! * **Zero unsafe code.** This library uses `#![forbid(unsafe_code)]` and was motivated by
//!     the complexity and amount of memory errors present in many alternatives.
//!
//! * **Intuitive API.** Uses similar or same methods as `std` when possible.
//!
//! * **Small footprint.** The core logic is <100 lines of code. The only dependencies are
//!     `hashbrown` (which `std` uses) and `parking_lot`.
//!
//! * **Really fast.** This implementation may be a more performant choice than some
//!     of the most popular concurrent hashmaps out there. Try it on your workload and let us know.
//!
//! ### See Also
//!
//! - **[contrie](https://crates.io/crates/contrie)** - A concurrent hash-trie map & set.
//! - **[dashmap](https://github.com/xacrimon/dashmap)** - Blazing fast concurrent HashMap for Rust.
//! - **[flurry](https://github.com/jonhoo/flurry)** - A port of Java's `java.util.concurrent.ConcurrentHashMap` to Rust. (Also part of a live stream series)
//!
//! ## Quick Start
//!
//! ```toml
//! [dependencies]
//! sharded = "0.3"
//! ```
//! ### Examples
//!
//! **Insert and retrieve values**
//!
//! ```
//! # use sharded::ConcurrentHashMap;
//! let users = ConcurrentHashMap::new();
//! users.insert(32, "Henry");
//! assert_eq!(&"Henry", users.get(32).unwrap());
//! ```
//!
//! ```
//! **Determine if a method will block**
//!
//! `try_get`, `try_insert`, etc, are available for avoiding blocks or situations that could
//! deadlock.
//!
//! ```
//! # use sharded::ConcurrentHashMap;
//! # let tokens = ConcurrentHashMap::new();
//! # tokens.insert(32, "tokendata");
//! # struct WouldBlock;
//! match tokens.try_get(&32) {
//!     Some(token) => Ok(token),
//!     None => Err(WouldBlock)
//! };
//! ```
//!
//! ## Performance Comparison
//!
//! These measurements were generated using [`jonhoo/bustle`](https://github.com/jonhoo/bustle). To reproduce the charts,
//! see the `benchmarks` directory. Benchmarks can be misleading. It is recommended to benchmark using a real application
//! workload.
//!
//! ### Average Performance by Implementation
//!
//! This ran each implementation over the presets in [`bustle::Mix`](https://docs.rs/bustle/0.4.2/bustle/struct.Mix.html) for 5
//! iterations / random seeds using a Intel® Core™ i9-9820X. Lower numbers are better. Approaches using a single `std::sync` Lock and `chashmap` were discarded for clarity (they are
//! a lot slower). If you know why `chashmap` is so slow in this test, please help here.
//!
//! ![Read Heavy Performance)](benchmarks/avg_performance_read_heavy.png)
//! ![Write Heavy Performance)](benchmarks/avg_performance_write_heavy.png)
//! ![Update Heavy Performance)](benchmarks/avg_performance_update_heavy.png)
//! ![Uniform Performance)](benchmarks/avg_performance_uniform.png)
//!
//! ## Acknowledgements
//!
//! Many thanks to
//!
//! - [Reddit community](https://www.reddit.com/r/rust) for a few pointers and
//! some motivation to take this project further.
//!
//! - [Jon Gjengset](https://github.com/jonhoo) for the live streams and utility crates involved
//!
//! - and countless OSS contributors that made this work possible
//!
//! ## License
//!
//! Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
//! 2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
//!
//! Unless you explicitly state otherwise, any contribution intentionally submitted
//! for inclusion in `sharded` by you, as defined in the Apache-2.0 license, shall be
//! dual licensed as above, without any additional terms or conditions.
#![forbid(unsafe_code)]

use hashbrown::raw::{RawIntoIter, RawTable};
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};
use std::borrow::Borrow;
use std::convert::TryInto;
use std::hash::{BuildHasher, Hash};
use std::{fmt, fmt::Debug};

use std::collections::hash_map::RandomState;

/// Number of shards
const DEFAULT_SHARD_COUNT: usize = 128;

// From hashbrown
// Ensures that a single closure type across uses of this which, in turn prevents multiple
// instances of any functions like RawTable::reserve from being generated
#[inline]
fn equivalent_key<K, V>(k: &K) -> impl Fn(&(K, V)) -> bool + '_
where
    K: Eq,
{
    move |x| k.eq(x.0.borrow())
}

// From hashbrown
#[inline]
fn make_hash<K, S>(hash_builder: &S, val: &K) -> u64
where
    K: Hash,
    S: BuildHasher,
{
    hash_builder.hash_one(val)
}

// From hashbrown
// Ensures that a single closure type across uses of this which, in turn prevents multiple
// instances of any functions like RawTable::reserve from being generated
#[inline]
fn make_hasher<K, V, S>(hash_builder: &S) -> impl Fn(&(K, V)) -> u64 + '_
where
    K: Hash,
    S: BuildHasher,
{
    move |val| make_hash::<K, S>(hash_builder, &val.0)
}

//#[inline]
//fn to_read_guard_iter<'a, K, V, S>(
//    guard: RwLockReadGuard<'a, Shard<K, V, S>>,
//) -> MappedRwLockReadGuard<'a, RawIter<(K, V)>> {
//    RwLockReadGuard::map(guard, |guard| {
//        // # Safety
//        //
//        // From `hashbrown`:
//        // > Returns an iterator over every element in the table. It is up to
//        // > the caller to ensure that the `RawTable` outlives the `RawIter`.
//        // > Because we cannot make the `next` method unsafe on the `RawIter`
//        // > struct, we have to make the `iter` method unsafe.
//        //
//        // We give Iter lifetime 'i and our lifetime for `RawTable` 's,
//        // which lives _at least_ as long as 'i
//        #[allow(unsafe_code)]
//        unsafe {
//            &guard.inner.iter()
//        }
//    })
//}

/// A concurrent lock-based `HashMap` based on `hashbrown` and `parking_lot`.
pub struct ConcurrentHashMap<K, V, S = RandomState, const N: usize = DEFAULT_SHARD_COUNT> {
    hash_builder: S,
    shards: [RwLock<Shard<K, V, S>>; N],
}

impl<K, V> ConcurrentHashMap<K, V, RandomState, DEFAULT_SHARD_COUNT> {
    /// Creates an empty `ConcurrentHashMap`.
    ///
    /// The hash map is initially created with a capacity of 0, so it will not allocate until it
    /// is first inserted into.
    ///
    /// # Examples
    ///
    /// ```
    /// use sharded::ConcurrentHashMap;
    /// let mut map: ConcurrentHashMap<&str, i32> = ConcurrentHashMap::new();
    /// ```
    #[must_use]
    pub fn new() -> ConcurrentHashMap<K, V, RandomState> {
        Default::default()
    }

    /// Creates an empty `ConcurrentHashMap` with the specified capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// use sharded::ConcurrentHashMap;
    /// let mut map: ConcurrentHashMap<&str, i32> = ConcurrentHashMap::with_capacity(100_000);
    /// ```
    #[inline]
    #[must_use]
    pub fn with_capacity(
        capacity: usize,
    ) -> ConcurrentHashMap<K, V, RandomState, DEFAULT_SHARD_COUNT> {
        ConcurrentHashMap::<_, _, _, DEFAULT_SHARD_COUNT>::with_capacity_and_hasher(
            capacity,
            RandomState::default(),
        )
    }
}

impl<K, V, S: BuildHasher, const N: usize> ConcurrentHashMap<K, V, S, N> {
    /// Creates an empty `ConcurrentHashMap` which will use the given hash builder to hash
    /// keys.
    ///
    /// The created map has the default initial capacity.
    ///
    /// Warning: `hash_builder` is normally randomly generated, and
    /// is designed to allow HashMaps to be resistant to attacks that
    /// cause many collisions and very poor performance. Setting it
    /// manually using this function can expose a DoS attack vector.
    ///
    /// The `hash_builder` passed should implement the [`BuildHasher`] trait for
    /// the HashMap to be useful, see its documentation for details.
    ///
    /// # Examples
    ///
    /// ```
    /// use sharded::ConcurrentHashMap;
    /// use std::collections::hash_map::RandomState;
    ///
    /// let mut map = ConcurrentHashMap::with_hasher(RandomState::new());
    /// map.insert(1, 2);
    /// ```
    #[inline]
    pub fn with_hasher(hash_builder: S) -> ConcurrentHashMap<K, V, S, N>
    where
        S: Clone,
    {
        ConcurrentHashMap::<_, _, _, N>::with_capacity_and_hasher(0, hash_builder)
    }

    /// Creates an empty `ConcurrentHashMap` with the specified capacity, using `hash_builder`
    /// to hash the keys.
    ///
    /// The hash map will be able to hold approximately `capacity` elements without
    /// reallocating. If `capacity` is 0, the hash map will not allocate.
    ///
    /// Warning: `hash_builder` is normally randomly generated, and
    /// is designed to allow HashMaps to be resistant to attacks that
    /// cause many collisions and very poor performance. Setting it
    /// manually using this function can expose a DoS attack vector.
    ///
    /// The `hash_builder` passed should implement the [`BuildHasher`] trait for
    /// the HashMap to be useful, see its documentation for details.
    ///
    /// # Examples
    ///
    /// ```
    /// use sharded::ConcurrentHashMap;
    /// use std::collections::hash_map::RandomState;
    ///
    /// let s = RandomState::new();
    /// let mut map = ConcurrentHashMap::with_capacity_and_hasher(10, s);
    /// map.insert(1, 2);
    /// ```
    pub fn with_capacity_and_hasher(
        capacity: usize,
        hash_builder: S,
    ) -> ConcurrentHashMap<K, V, S, N>
    where
        S: Clone,
    {
        // per shard capacity
        let capacity = (capacity + DEFAULT_SHARD_COUNT - 1) / DEFAULT_SHARD_COUNT;

        let shards: Vec<RwLock<Shard<K, V, S>>> =
            std::iter::repeat(|| RawTable::with_capacity(capacity))
                .map(|f| f())
                .take(DEFAULT_SHARD_COUNT)
                .map(|inner| {
                    RwLock::new(Shard {
                        inner,
                        hash_builder: hash_builder.clone(),
                    })
                })
                .collect::<Vec<_>>();

        match shards.try_into() {
            Ok(shards) => ConcurrentHashMap {
                hash_builder,
                shards,
            },
            // .unwrap() requires Debug
            // this never panics because the iter takes exactly DEFAULT_SHARD_COUNT
            Err(_) => panic!("unable to build inner"),
        }
    }

    /// Returns the approximate number of elements the map can hold without reallocating.
    ///
    /// **Locks** - Acquires a read lock on one of `N` shards.
    ///
    /// # Examples
    ///
    /// ```
    /// use sharded::ConcurrentHashMap;
    /// let map: ConcurrentHashMap<i32, i32> = ConcurrentHashMap::with_capacity(100);
    /// assert!(map.capacity() >= 100);
    /// ```
    #[inline]
    pub fn capacity(&self) -> usize {
        self.shards.first().unwrap().read().inner.capacity()
    }

    ///// An iterator visiting all key-value pairs in arbitrary order.
    ///// The iterator element type is `(&'a K, &'a V)`.
    /////
    ///// **Locks** - [`Iter`] acquires a read lock on one of the `N` shards at a time.
    /////
    ///// # Examples
    /////
    ///// ```
    ///// use sharded::ConcurrentHashMap;
    /////
    ///// let map = ConcurrentHashMap::from([
    /////     ("a", 1),
    /////     ("b", 2),
    /////     ("c", 3),
    ///// ]);
    /////
    ///// for key in map.keys() {
    /////     println!("{key}");
    ///// }
    ///// ```
    //pub fn iter<'i, 's: 'i>(&'s self) -> Iter<'i, K, V, S> {
    //    let first = self.shards.first().unwrap();

    //    let iter = to_read_guard_iter(first.read());

    //    let rest = self.shards.split_at(1).1.iter().collect();

    //    Iter { iter, rest }
    //}

    //    /// Creates a consuming iterator, that is, one that moves each key-value
    //    /// pair out of the map in arbitrary order. The map cannot be used after
    //    /// calling this. Yields the values of the map.
    //    pub fn into_values(self) -> IntoValues<K, V> {
    //        IntoValues {
    //            iter: self.into_iter(),
    //        }
    //    }

    /// Returns a guarded reference for the value corresponding to the
    /// provided key.
    ///
    /// The key may be any borrowed form of the map's key type, but
    /// [`Hash`] and [`Eq`] on the borrowed form *must* match those for
    /// the key type.
    ///
    /// # Examples
    ///
    /// ```
    /// use sharded::ConcurrentHashMap;
    ///
    /// let mut map = ConcurrentHashMap::new();
    /// map.insert(1, "a");
    /// assert_eq!(map.get(&1), Some(&"a"));
    /// assert_eq!(map.get(&2), None);
    /// ```
    #[inline]
    pub fn get<'a>(&'a self, key: &'a K) -> Option<MappedRwLockReadGuard<'_, V>>
    where
        K: Hash + Eq,
    {
        let hash = make_hash::<K, _>(&self.hash_builder, key);

        let i = hash as usize % N;

        let shard = match self.shards.get(i) {
            Some(lock) => lock.read(),
            None => panic!("index out of bounds"),
        };

        RwLockReadGuard::try_map(shard, |shard| {
            match shard.inner.get(hash, equivalent_key(key)) {
                Some((_, v)) => Some(v),
                _ => None,
            }
        })
        .ok()
    }

    //    /// Applies given function to the value if it exists,
    //    /// otherwise returns `None`.
    //    #[inline]
    //    pub fn get_then<'a, U, F>(&'a self, key: &'a K, func: F) -> Option<U>
    //    where
    //        K: Hash + Eq,
    //        F: FnOnce(&V) -> U,
    //    {
    //        let hash = make_hash::<K, _>(&self.hash_builder, key);
    //
    //        let i = (hash as usize) % N;
    //
    //        let shard = match self.shards.get(i) {
    //            Some(lock) => lock.read(),
    //            None => panic!("index out of bounds"),
    //        };
    //
    //        shard
    //            .inner
    //            .get(hash, equivalent_key(key))
    //            .map(|kv| func(&kv.1))
    //    }
    //
    //    /// Returns a cloned value corresponding to the provided key
    //    #[inline]
    //    pub fn get_owned<'a>(&'a self, key: &'a K) -> Option<V>
    //    where
    //        K: Eq + Hash,
    //        V: Clone,
    //    {
    //        let (key, shard) = self.read(key);
    //        shard.get(key).cloned()
    //    }

    ///// Does the map contain the provided key
    //#[inline]
    //pub fn contains<'a>(&'a self, key: &'a K) -> bool
    //where
    //    K: Eq + Hash,
    //{
    //    let (key, shard) = self.read(key);
    //    shard.get(key).is_some()
    //}

    ///// Number of elements in the map
    //#[inline]
    //pub fn len(&self) -> usize {
    //    return self.shards.iter().map(|x| x.read().len()).sum();
    //}

    ///// Is `len == 0`
    //#[inline]
    //pub fn is_empty(&self) -> bool {
    //    self.len() == 0
    //}

    /// Insert a key value pair into the Map. Returns the existing
    /// value at the provided key if there was one.
    #[inline]
    pub fn insert(&self, k: K, v: V) -> Option<V>
    where
        K: Hash + Eq,
    {
        let hash = make_hash::<K, _>(&self.hash_builder, &k);

        let i = hash as usize % N;

        let mut shard = match self.shards.get(i) {
            Some(lock) => lock.write(),
            None => panic!("index out of bounds"),
        };

        shard.insert(hash, k, v)
    }

    ///// Remove using the provided key. Returns the existing value, if any.
    //pub fn remove(&self, k: K) -> Option<V>
    //where
    //    K: Hash + Eq,
    //{
    //    let hash = make_hash::<K, _>(&self.hash_builder, &k);

    //    let i = hash as usize % N;

    //    let shard = match self.shards.get(i as usize) {
    //        Some(lock) => lock.write(),
    //        None => panic!("index out of bounds"),
    //    };

    //    shard.remove(hash, k)
    //}
}

impl<K, V, S, const N: usize> Default for ConcurrentHashMap<K, V, S, N>
where
    S: Default + BuildHasher + Clone,
{
    /// Creates an empty `ConcurrentHashMap<K, V, S, N>`, with the `Default` value for the hasher
    /// and 128 for the Shard Count.
    #[inline]
    fn default() -> ConcurrentHashMap<K, V, S, N> {
        if N == 0 {
            panic!("number of shards must be > 0")
        }
        ConcurrentHashMap::<K, V, S, N>::with_hasher(Default::default())
    }
}
//
//impl<K: 'static, V: 'static> IntoIterator for Map<K, V> {
//    type IntoIter = IntoIter<K, V>;
//
//    type Item = (K, V);
//
//    /// Creates a consuming iterator, that is, one that moves each key-value
//    /// pair out of the map in arbitrary order. The map cannot be used after
//    /// calling this.
//    fn into_iter(self) -> IntoIter<K, V> {
//        let shards: Vec<_> = self.shards.into();
//
//        let mut shards: Vec<Shard<K, V>> = shards.into_iter().map(|s| s.into_inner()).collect();
//
//        IntoIter {
//            iter: shards.pop().unwrap().inner.into_iter(),
//            shards,
//        }
//    }
//}
//

///// An iterator over the entries of a `ConcurrentHashMap`.
/////
///// This `struct` is created by the [`iter`] method on [`ConcurrentHashMap`]. See its
///// documentation for more.
/////
///// [`iter`]: ConcurrentHashMap::iter
/////
///// # Example
/////
///// ```
///// use sharded::ConcurrentHashMap;
/////
///// let map = ConcurrentHashMap::from([
/////     ("a", 1),
///// ]);
///// let iter = map.iter();
///// ```
//pub struct Iter<'a, K: 'a, V: 'a, S: 'a> {
//    iter: MappedRwLockReadGuard<'a, RawIter<(K, V)>>,
//    rest: Vec<&'a RwLock<Shard<K, V, S>>>,
//}

//impl<'a, K, V, S> Iterator for Iter<'a, K, V, S> {
//    type Item = (&'a K, &'a V);
//
//    #[inline]
//    fn next(&mut self) -> Option<(&'a K, &'a V)> {
//        // Avoid `Option::map` because it bloats LLVM IR.
//        match self.iter.next() {
//            #[allow(unsafe_code)]
//            // # Safety
//            // Guard is held and we are still guaranteed
//            // that RawTable outlives the bucket ref
//            Some(x) => unsafe {
//                let r = x.as_ref();
//                Some((&r.0, &r.1))
//            },
//            None => match self.rest.pop() {
//                Some(s) => {
//                    self.iter = to_read_guard_iter(s.read());
//                    self.next()
//                }
//                None => None,
//            },
//        }
//    }
//
//    #[inline]
//    fn size_hint(&self) -> (usize, Option<usize>) {
//        (self.iter.size_hint().0, None)
//    }
//}

/**
// An iterator over the keys of a `ConcurrentHashMap`.
//
// This `struct` is created by the ~~ConcurrentHashMap::keys~~ method on [`ConcurrentHashMap`]. See its
// documentation for more.
//
// [`keys`]: ConcurrentHashMap::keys
//
// # Example
//
// ```
// use sharded::ConcurrentHashMap;
//
// let map = ConcurrentHashMap::from([
//     ("a", 1),
// ]);
// let iter_keys = map.keys();
// ```
//pub struct Keys<'a, K: 'a, V: 'a, S: 'a> {
//    iter: Iter<'a, K, V, S>,
//}
//
//impl<'a, K, V, S> Iterator for Keys<'a, K, V, S> {
//    type Item = &'a K;
//
//    #[inline]
//    fn next(&mut self) -> Option<&'a K> {
//        self.iter.next().map(|pair| pair.0)
//    }
//
//    #[inline]
//    fn size_hint(&self) -> (usize, Option<usize>) {
//        self.iter.size_hint()
//    }
//}
**/

/// An owning iterator over the entries of a `ConcurrentHashMap`.
///
/// This `struct` is created by the [`into_iter`] method on [`ConcurrentHashMap`]
/// (provided by the [`IntoIterator`] trait). See its documentation for more.
///
/// [`into_iter`]: IntoIterator::into_iter
/// [`IntoIterator`]: crate::iter::IntoIterator
///
/// # Example
///
/// ```
/// use sharded::ConcurrentHashMap;
///
/// let map = ConcurrentHashMap::from([
///     ("a", 1),
/// ]);
/// let iter = map.into_iter();
/// ```
pub struct IntoIter<K: 'static, V: 'static> {
    iter: RawIntoIter<(K, V)>,
    shards: Vec<Shard<K, V>>,
}

pub struct IntoValues<K: 'static, V: 'static> {
    iter: IntoIter<K, V>,
}

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);

    #[inline]
    fn next(&mut self) -> Option<(K, V)> {
        match self.iter.next() {
            Some(item) => Some(item),
            None => match self.shards.pop() {
                Some(s) => {
                    self.iter = s.inner.into_iter();
                    self.next()
                }
                None => None,
            },
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.iter.size_hint().0, None)
    }
}

impl<K, V> Iterator for IntoValues<K, V> {
    type Item = V;

    #[inline]
    fn next(&mut self) -> Option<V> {
        self.iter.next().map(|(_, v)| v)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.iter.size_hint().0, None)
    }
}

//

//use std::iter::Extend;
//
//impl<K, V> Extend<(K, V)> for Map<K, V>
//where
//    K: Hash + Eq + Send + Sync + 'static,
//    V: Send + Sync + 'static,
//{
//    fn extend<T>(&mut self, iter: T)
//    where
//        T: IntoIterator<Item = (K, V)>,
//    {
//        let iter = iter.into_iter();
//        // iter.size_hint()
//
//        let t_handles = Vec::with_capacity(DEFAULT_SHARD_COUNT as usize);
//        let txs = Vec::with_capacity(DEFAULT_SHARD_COUNT as usize);
//
//        for i in 0..DEFAULT_SHARD_COUNT {
//            let shard = self.shards[i as usize].write().unwrap();
//            let shard = std::sync::Arc::new(shard);
//            // ^ need crossbeam probably
//            let (tx, rx) = std::sync::mpsc::channel();
//            txs.push(tx);
//
//            std::thread::spawn(move || {
//                for (key, value) in rx {
//                    shard.insert(key, value);
//                }
//            });
//        }
//
//        let (rx, tx) = std::sync::mpsc::channel();
//    }
//}

/// A single shard in the map
#[derive(Clone)]
pub(crate) struct Shard<K, V, S = RandomState> {
    hash_builder: S,
    inner: RawTable<(K, V)>,
}

impl<K, V> Debug for Shard<K, V>
where
    K: Debug,
    V: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("shard").finish()
    }
}

#[allow(dead_code)]
impl<K, V, S> Shard<K, V, S>
where
    S: BuildHasher,
{
    /// Number of items in the shard
    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.inner.len()
    }

    /// Is `len == 0`
    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.inner.len() == 0
    }

    /// Remove the key, returning the value at that position if it existed
    #[inline]
    pub(crate) fn remove(&mut self, hash: u64, key: K) -> Option<V>
    where
        K: Hash + Eq,
    {
        #[allow(clippy::manual_map)] // reduce compiler IR, I think!
        match self.inner.remove_entry(hash, equivalent_key(&key)) {
            Some((_, v)) => Some(v),
            None => None,
        }
    }

    /// Get mutable value for the provided key
    #[inline]
    pub(crate) fn get_mut(&mut self, hash: u64, key: &K) -> Option<&mut V>
    where
        K: Hash + Eq,
    {
        match self.inner.get_mut(hash, equivalent_key(key)) {
            Some(&mut (_, ref mut v)) => Some(v),
            None => None,
        }
    }

    /// Insert the key value pair
    #[inline]
    pub(crate) fn insert(&mut self, hash: u64, key: K, v: V) -> Option<V>
    where
        K: Hash + Eq,
    {
        if let Some((_, item)) = self.inner.get_mut(hash, equivalent_key(&key)) {
            Some(std::mem::replace(item, v))
        } else {
            self.inner
                .insert(hash, (key, v), make_hasher::<K, V, S>(&self.hash_builder));
            None
        }
    }

    /// Get the value for the key if it exists
    #[inline]
    pub(crate) fn get(&self, hash: u64, key: &K) -> Option<&V>
    where
        K: Hash + Eq,
    {
        match self.inner.get(hash, equivalent_key(key)) {
            Some((_, v)) => Some(v),
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn test_insert_values() {
        let map = ConcurrentHashMap::new();
        {
            map.insert("k", "v");
        }
        assert_eq!(*map.get(&"k").unwrap(), "v");
    }

    #[test]
    fn test_other_deadlock() {
        let map_1 = Arc::new(ConcurrentHashMap::<i32, String>::default());
        let map_2 = map_1.clone();

        for i in 0..1000 {
            map_1.insert(i, "foobar".to_string());
        }

        let _writer = std::thread::spawn(move || loop {
            println!("writer iteration");
            for i in 0..1000 {
                map_1.insert(i, "foobaz".to_string());
            }
        });

        let _reader = std::thread::spawn(move || loop {
            println!("reader iteration");
            for i in 0..1000 {
                let j = i32::min(i + 100, 1000);
                let rng: Vec<i32> = (i..j).collect();
                let _v: Vec<_> = rng.iter().map(|k| map_2.get(k)).collect();
            }
        });

        std::thread::sleep(Duration::from_secs(10));
    }
}
