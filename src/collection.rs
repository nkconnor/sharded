use crate::shard::ExtractShardKey;
use crate::{HashMap, HashSet};
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
    fn insert(&mut self, v: Value) -> Option<Value>;

    fn get(&self, k: &K) -> Option<Value>;

    /// Returns the count of values stored in the collection
    fn len(&self) -> usize;

    /// Returns the current specified capacity
    fn capacity(&self) -> usize;
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

    fn insert(&mut self, v: (K, V)) -> Option<(K, V)> {
        // this should not clone and just return V
        // which I think means another type param or some
        // special
        HashMap::<K, V, S>::insert(self, v.0.clone(), v.1.clone()).map(|prior| (v.0, prior))
    }

    fn get(&self, _k: &K) -> Option<(K, V)> {
        todo!()
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn capacity(&self) -> usize {
        self.capacity()
    }
}

impl<K, S> Collection<K, K> for HashSet<K, S>
where
    K: Hash + Clone + Eq,
    S: BuildHasher + Clone + Default,
{
    fn with_capacity(capacity: usize) -> Self {
        HashSet::<K, S>::with_capacity_and_hasher(capacity, S::default())
    }

    fn insert(&mut self, v: K) -> Option<K> {
        if HashSet::<K, S>::insert(self, v.clone()) {
            Some(v.clone())
        } else {
            None
        }
    }

    fn get(&self, _k: &K) -> Option<K> {
        todo!()
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn capacity(&self) -> usize {
        self.capacity()
    }
}
