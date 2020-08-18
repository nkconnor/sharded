//! A hash-based RwLock for performant read/writes on hash-based
//! collections like HashMap, HashSet.
#![deny(unsafe_code)]
#![allow(dead_code)]
#![allow(unused_macros)]

use parking_lot;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::Hash;

//trait Lock {
//    type Inner;
//    fn new(t: Self::Inner) -> Self;
//
//    fn read(&self) -> Box<dyn Deref<Target = Self::Inner>>;
//}
//
//
//
//    impl<T> Lock for RwLock<T> {
//        type Inner = T;
//
//        fn new(t: T) -> Self {
//            parking_lot::RwLock::new(t)
//        }
//
//        fn read(&self) -> Box<dyn Deref<Target = T> + '_> {
//            Box::new(self.read())
//        }
//    }

pub trait ExtractShardKey<K: Hash> {
    fn key(&self) -> &K;
}

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

pub struct Sharded<T> {
    shards: Vec<T>,
}

impl<T> Sharded<T> {
    pub fn new<K, V, U>(inner: U) -> Sharded<parking_lot::RwLock<U>>
    where
        K: Hash,
        V: ExtractShardKey<K>,
        U: Collection<K, V>, // L: Lock<Inner = U>,
    {
        let shard_count = 100;

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

        let shards = shards
            .into_iter()
            .map(|shard| parking_lot::RwLock::new(shard))
            .collect();

        Sharded { shards }
    }

    fn index<K: Hash>(k: &K) -> usize {
        let shard_count = 100;
        use std::hash::Hasher;
        let mut s = DefaultHasher::new();
        k.hash(&mut s);
        (s.finish() as usize % shard_count) as usize
    }
}

impl<T> Sharded<parking_lot::RwLock<T>> {
    pub fn write<K: Hash>(&self, k: &K) -> Option<parking_lot::RwLockWriteGuard<'_, T>> {
        let i = Self::index(k);
        self.shards.get(i).map(|lock| lock.write())
    }

    pub fn read<K: Hash>(&self, k: &K) -> Option<parking_lot::RwLockReadGuard<'_, T>> {
        let i = Self::index(k);
        self.shards.get(i).map(|lock| lock.read())
    }
}

macro_rules! shard {
    ($($arg:tt)*) => {{
        Sharded::<()>::new($($arg)*)
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
            .unwrap()
            .insert("key".to_string(), "value".to_string());

        assert_eq!(
            x.read(&"key".to_string())
                .unwrap()
                .get(&"key".to_string())
                .unwrap(),
            "value"
        );
    }

    #[test]
    fn hold_read_and_write() {
        let map = shard!(HashMap::new());

        let mut write = map.write(&"abc".to_string()).unwrap();
        write.insert("abc".to_string(), "asdf".to_string());

        let _read = map.read(&"asdfas".to_string()).unwrap();
        let _read_too = map.read(&"asdfas".to_string()).unwrap();
        assert!(_read.is_empty());
    }
}
