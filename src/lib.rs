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
#![deny(unsafe_code)]
#![allow(dead_code)]
#![allow(unused_macros)]
#![feature(generic_associated_types)]
#![feature(in_band_lifetimes)]

use parking_lot;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::Hash;

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

impl<T> Lock<T> for parking_lot::RwLock<T> {
    #[rustfmt::skip]
    type ReadGuard<'b> where T: 'b = parking_lot::RwLockReadGuard<'b, T>;
    #[rustfmt::skip]
    type WriteGuard<'b> where T: 'b = parking_lot::RwLockWriteGuard<'b, T>;

    fn new(t: T) -> Self {
        parking_lot::RwLock::new(t)
    }

    fn read(&self) -> Self::ReadGuard<'_> {
        self.read()
    }

    fn write(&self) -> Self::WriteGuard<'_> {
        self.write()
    }
}

/// Teases out the sharding key for example
/// from an IntoIterator value.
pub trait ExtractShardKey<K: Hash> {
    fn key(&self) -> &K;
}

/// Basic methods needing implemented for shard construction
pub trait Collection<K: Hash, Value: ExtractShardKey<K>>:
    IntoIterator<Item = Value> + Clone
{
    fn with_capacity(capacity: usize) -> Self;

    fn insert(&mut self, v: Value);

    fn len(&self) -> usize;
}

impl<K: Hash, V> ExtractShardKey<K> for (K, V) {
    fn key(&self) -> &K {
        &self.0
    }
}

impl<K: Hash + Clone + Eq, V: Clone> Collection<K, (K, V)> for HashMap<K, V> {
    fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity(capacity)
    }

    fn insert(&mut self, v: (K, V)) {
        HashMap::insert(self, v.0, v.1);
    }

    fn len(&self) -> usize {
        self.len()
    }
}

/// The sharded lock collection.
pub struct Shard<T> {
    shards: Vec<T>,
}

impl<T> Shard<T> {
    pub fn new<K, V, U, L>(inner: U) -> Shard<L>
    where
        K: Hash,
        V: ExtractShardKey<K>,
        U: Collection<K, V>,
        L: Lock<U>,
    {
        let shard_count = 250;

        let mut shards = vec![U::with_capacity(inner.len() / shard_count); shard_count];

        inner.into_iter().for_each(|item| {
            // for each item, push it to the appropriate shard
            let i = Self::index(item.key());
            // Safe because we just initialized shards to `shard_count`
            // and hash % `shard_count` must be bounded by `shard_count`
            #[allow(unsafe_code)]
            unsafe {
                shards.get_unchecked_mut(i).insert(item)
            }
        });

        let shards = shards.into_iter().map(|shard| L::new(shard)).collect();

        Shard { shards }
    }

    fn index<K: Hash>(k: &K) -> usize {
        let shard_count = 250;
        use std::hash::Hasher;
        let mut s = DefaultHasher::new();
        k.hash(&mut s);
        (s.finish() as usize % shard_count) as usize
    }
}

//trait ShardOps {
//    type K: Hash;
//    type V;
//    type U: Collection<Self::K, Self::V>;
//    type L: Lock<Self::U>;
//
//    fn index(k: &Self::K) -> usize;
//
//    fn shards(&self) -> Vec<Self::L>;
//
//    fn write(&'a self, k: &Self::K) -> <<Self as ShardOps>::L as Lock<Self::U>>::WriteGuard<'a> {
//        //let i = Self::index(k);
//        //let l = *self.shards().get(i).unwrap();
//        //l.write()
//        //
//        todo!()
//    }
//}

impl<T> Shard<parking_lot::RwLock<T>> {
    pub fn write<K: Hash>(&self, k: &K) -> parking_lot::RwLockWriteGuard<'_, T> {
        let i = Self::index(k);
        self.shards.get(i).map(|lock| lock.write()).unwrap()
    }

    pub fn read<K: Hash>(&self, k: &K) -> parking_lot::RwLockReadGuard<'_, T> {
        let i = Self::index(k);
        self.shards.get(i).map(|lock| lock.read()).unwrap()
    }
}

/// So you don't have to turbofish
#[macro_export]
macro_rules! shard {
    ($($arg:tt)*) => {{
        Shard::<()>::new($($arg)*)
    }}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn read_and_write() {
        let x = shard!(HashMap::new());

        x.write(&"key".to_string())
            .insert("key".to_string(), "value".to_string());

        assert_eq!(
            x.read(&"key".to_string()).get(&"key".to_string()).unwrap(),
            "value"
        );
    }

    #[test]
    fn hold_read_and_write() {
        let map = shard!(HashMap::new());

        let mut write = map.write(&"abc".to_string());
        write.insert("abc".to_string(), "asdf".to_string());

        let _read = map.read(&"asdfas".to_string());
        let _read_too = map.read(&"asdfas".to_string());
        assert!(_read.is_empty());
    }
}
