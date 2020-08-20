use crate::shard::ExtractShardKey;
use std::collections::HashMap;
use std::hash::Hash;

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
