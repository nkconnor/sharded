//! A generic sharded locking mechanism for hash based collections to speed up concurrent reads/writes. `Shard::new` splits
//! the underlying collection into N shards each with its own lock. Calling `read(key)` or `write(key)`
//! returns a guard for only a single shard. The underlying locks should be generic, so you can use
//! it with any `Mutex` or `RwLock` in `std::sync` or `parking_lot`.
//!
//! In a probably wrong and unscientific test of concurrent readers/single writer,
//! `shard_lock` is **100-∞∞x faster** (deadlocks?) than [`dashmap`](https://github.com/xacrimon/dashmap), and
//! **13x faster** than a single `parking_lot::RwLock`. Carrying `Shard<RwLock<T>>` is possibly more obvious
//! and simpler than other approaches. The library has a very small footprint at ~100 loc and optionally no
//! dependencies.
//!
//! `shard_lock` is flexible enough to shard any hash based collection such as `HashMap`, `HashSet`, `BTreeMap`, and `BTreeSet`.
//!
//! _**Warning:** shard_lock is in early development and unsuitable for production. The API is undergoing changes and is not dependable._
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(unused_macros)]
#![allow(incomplete_features)]
#![feature(generic_associated_types)]
#![feature(in_band_lifetimes)]

#[cfg(feature = "3rd-party")]
use ahash::AHasher as DefaultHasher;

#[cfg(not(feature = "3rd-party"))]
use std::collections::hash_map::DefaultHasher;

use std::hash::Hasher;

#[cfg(feature = "3rd-party")]
use hashbrown::HashMap;

#[cfg(feature = "3rd-party")]
use hashbrown::HashSet;

#[cfg(not(feature = "3rd-party"))]
use std::collections::HashMap;

#[cfg(not(feature = "3rd-party"))]
use std::collections::HashSet;

#[cfg(feature = "3rd-party")]
use parking_lot;

#[cfg(feature = "3rd-party")]
mod parking_lock;

#[cfg(not(feature = "3rd-party"))]
mod std_lock;

#[cfg(feature = "3rd-party")]
pub type RwLock<T> = parking_lot::RwLock<T>;

#[cfg(not(feature = "3rd-party"))]
pub type RwLock<T> = std::sync::RwLock<T>;

/// Sharded lock-based concurrent map using the crate default lock and map implementations.
pub type Map<K, V> = Shard<RwLock<HashMap<K, V>>>;

/// Sharded lock-based concurrent set using the crate default lock and set implementations.
pub type Set<K> = Shard<RwLock<HashSet<K>>>;

use std::hash::Hash;

// Global shard count for collections
// TODO configurable via construction
const SHARD_COUNT: usize = 128;

/// Generic locking implementation.
pub trait Lock<T> {
    #[rustfmt::skip]
    type ReadGuard<'a> where T: 'a;
    #[rustfmt::skip]
    type WriteGuard<'a> where T: 'a;

    fn new(t: T) -> Self;

    fn write(&self) -> Self::WriteGuard<'_>;

    fn read(&self) -> Self::ReadGuard<'_>;
}

/// Teases out the sharding key for example
/// from an IntoIterator value.
pub trait ExtractShardKey<K: Hash> {
    fn key(&self) -> &K;
}

/// Basic methods needing implemented for shard construction
pub trait Collection<K, Value>: IntoIterator<Item = Value> + Clone
where
    K: Hash,
    Value: ExtractShardKey<K>,
{
    fn with_capacity(capacity: usize) -> Self;

    fn insert(&mut self, v: Value);

    fn len(&self) -> usize;

    fn capacity(&self) -> usize;
}

// Takes key from map iter values
impl<K: Hash, V> ExtractShardKey<K> for (K, V) {
    fn key(&self) -> &K {
        &self.0
    }
}

impl<K, V> Collection<K, (K, V)> for HashMap<K, V>
where
    K: Hash + Clone + Eq,
    V: Clone,
{
    fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity(capacity)
    }

    fn insert(&mut self, v: (K, V)) {
        HashMap::insert(self, v.0, v.1);
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn capacity(&self) -> usize {
        self.capacity()
    }
}

/// The sharded lock collection. This is the main data type in the crate. See also the type aliases
/// `Map`, `Set`, and so on.
///
/// # Examples
///
/// ```ignore
/// use sharded::Shard;
///
/// let users = Shard::from(HashMap::new());
///
/// let guard = users.read("uid-31356");
///
/// guard.get("uid-31356");
/// ```
pub struct Shard<T> {
    shards: Vec<T>,
}

impl<K: Hash> Shard<K> {
    /// Create a new shard from an existing collection
    pub fn from<V, U, L>(inner: U) -> Shard<L>
    where
        V: ExtractShardKey<K>,
        U: Collection<K, V>,
        L: Lock<U>,
    {
        let mut shards = vec![U::with_capacity(inner.len() / SHARD_COUNT); SHARD_COUNT];

        inner.into_iter().for_each(|item| {
            // for each item, push it to the appropriate shard
            let i = index(item.key());
            if let Some(shard) = shards.get_mut(i) {
                shard.insert(item)
            } else {
                panic!(
                    "We just initialized shards to `SHARD_COUNT` and hash % `SHARD_COUNT`
                    should be bounded"
                );
            }
        });

        let shards = shards.into_iter().map(|shard| L::new(shard)).collect();

        Shard { shards }
    }
}

fn index<K: Hash>(k: &K) -> usize {
    let mut s = DefaultHasher::default();
    k.hash(&mut s);
    (s.finish() as usize % SHARD_COUNT) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_and_write() {
        let x: Shard<RwLock<HashMap<String, String>>> = Shard::from(HashMap::new());

        x.write(&"key".to_string())
            .insert("key".to_string(), "value".to_string());

        assert_eq!(
            x.read(&"key".to_string()).get(&"key".to_string()).unwrap(),
            "value"
        );
    }

    #[test]
    fn hold_read_and_write() {
        let map = Shard::from(HashMap::new());

        let mut write = map.write(&"abc".to_string());
        write.insert("abc".to_string(), "asdf".to_string());

        let _read = map.read(&"asdfas".to_string());
        let _read_too = map.read(&"asdfas".to_string());
        assert!(_read.is_empty());
    }
}
