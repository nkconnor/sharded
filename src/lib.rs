//! A hash-based RwLock for performant read/writes on hash-based
//! collections like HashMap, HashSet.
use parking_lot;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

//trait Lock {
//    type Inner;
//    type ReadGuard<'r>;
//    type WriteGuard<'w>;
//    fn new(t: Self::Inner) -> Self;
//
//    fn read<'r>(&self) -> Self::ReadGuard<'r>;
//}
//
//impl<T> Lock for parking_lot::RwLock<T>
//where
//    for<'w> T: 'w,
//    for<'r> T: 'r,
//{
//    type Inner = T;
//    type ReadGuard = parking_lot::RwLockReadGuard<'r, T>;
//    type WriteGuard = parking_lot::RwLockWriteGuard<'w, T>;
//    fn new(t: T) -> Self {
//        parking_lot::RwLock::new(t)
//    }
//
//    fn read<'r>(&self) -> &Self::ReadGuard<'r> {
//        self.read()
//    }
//}

trait ExtractShardKey<K: Hash> {
    fn key(&self) -> &K;
}

trait Collection<K: Hash, Value: ExtractShardKey<K>>: IntoIterator<Item = Value> + Clone {
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
        std::collections::HashMap::insert(self, v.0, v.1);
    }

    fn len(&self) -> usize {
        self.len()
    }
}

struct Sharded<T> {
    shards: Vec<T>,
}

impl<T> Sharded<T> {
    fn new<K, V, U>(inner: U) -> Sharded<parking_lot::RwLock<U>>
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
            unsafe { shards.get_unchecked_mut(i).insert(item) }
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
    fn write<K: Hash>(&self, k: &K) -> Option<parking_lot::RwLockWriteGuard<'_, T>> {
        let i = Self::index(k);
        self.shards.get(i).map(|lock| lock.write())
    }
    fn read<K: Hash>(&self, k: &K) -> Option<parking_lot::RwLockReadGuard<'_, T>> {
        let i = Self::index(k);
        self.shards.get(i).map(|lock| lock.read())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::RwLock;
    use std::collections::HashMap;

    #[test]
    fn read_and_write() {
        let x = Sharded::<()>::new(HashMap::new());

        {
            x.write(&"key".to_string())
                .unwrap()
                .insert("key".to_string(), "value".to_string());
        }
        assert_eq!(
            x.read(&"key".to_string())
                .unwrap()
                .get(&"key".to_string())
                .unwrap(),
            "value"
        );

        assert_eq!(2 + 2, 4);
    }
}
