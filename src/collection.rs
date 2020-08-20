use crate::shard::ExtractShardKey;
use crate::HashMap;
use std::hash::{BuildHasher, Hash};

/// Basic methods needing implemented for shard construction
pub trait Collection<K, Value>: IntoIterator<Item = Value> + Clone
where
    K: Hash,
    Value: ExtractShardKey<K>,
{
    /// Creates an empty collection with specified capacity. Usually
    /// this means the collection should avoid resizing until that threshold
    /// is reached
    fn with_capacity(capacity: usize) -> Self;

    /// Insert a (possibly) key value pair into the collection
    fn insert(&mut self, v: Value);

    /// Returns the count of values stored in the collection
    fn len(&self) -> usize;

    /// Returns the current specified capacity
    fn capacity(&self) -> usize;
}

mod api {
    use crate::{HashMap, RwLock, Shard, ShardLock};
    use std::hash::{BuildHasher, Hash};
    //use std::ops::Deref;
    //use std::sync::RwLockReadGuard;

    // struct GuardMap<'a, 'g, T, U>(&'a T, RwLockReadGuard<'g, U>);

    // impl<'a, 'g, T, U> Deref for GuardMap<'a, 'g, T, U> {
    //     type Target = T;

    //     fn deref(&self) -> &T {
    //         self.0
    //     }
    // }

    impl<K, V, S> Shard<RwLock<HashMap<K, V, S>>>
    where
        K: Hash + Eq + Clone,
        S: BuildHasher + Clone + Default,
        V: Clone,
    {
        fn insert(&self, k: K, v: V) -> Option<V> {
            let mut guard = self.write(&k);
            guard.insert(k, v)
        }

        //    // use std::borrow::Borrow;
        //    // where K: Borrow<Q>,
        //    fn get<'a, 'b>(&'a self, key: &'b K) -> Option<GuardMap<'a, '_, V, HashMap<K, V>>> {
        //        let guard = self.read(key);
        //        // guard.map(|c| c.get()) ..

        //        guard.get(&key).map(|value| GuardMap(value, guard))
        //    }
    }
}

impl<K, V, S> Collection<K, (K, V)> for HashMap<K, V, S>
where
    K: Hash + Clone + Eq,
    V: Clone,
    S: BuildHasher + Clone + Default,
{
    fn with_capacity(capacity: usize) -> Self {
        HashMap::<K, V, S>::with_capacity_and_hasher(capacity, S::default())
    }

    fn insert(&mut self, v: (K, V)) {
        HashMap::<K, V, S>::insert(self, v.0, v.1);
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn capacity(&self) -> usize {
        self.capacity()
    }
}
