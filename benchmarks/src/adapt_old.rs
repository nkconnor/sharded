use bustle::*;
use dashmap::DashMap;
use sharded::{RwLock, Shard};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct ShardTable<K>(Arc<Shard<RwLock<HashMap<K, u32>>>>);

impl<K> Collection for ShardTable<K>
where
    K: Send + Sync + From<u64> + Copy + 'static + std::hash::Hash + Eq,
{
    type Handle = Self;
    fn with_capacity(capacity: usize) -> Self {
        let m: Shard<RwLock<HashMap<K, u32>>> = Shard::from(HashMap::with_capacity(capacity));

        let m = Arc::new(m);
        Self(m)
    }

    fn pin(&self) -> Self::Handle {
        self.clone()
    }
}

impl<K> CollectionHandle for ShardTable<K>
where
    K: Send + From<u64> + Copy + 'static + std::hash::Hash + Eq,
{
    type Key = K;

    fn get(&mut self, key: &Self::Key) -> bool {
        let guard = self.0.read::<Self::Key>(key);
        (*guard).get(key).is_some()
    }

    fn insert(&mut self, key: &Self::Key) -> bool {
        self.0.write(key).insert(*key, 0).is_none()
    }

    fn remove(&mut self, key: &Self::Key) -> bool {
        self.0.write(key).remove(key).is_some()
    }

    fn update(&mut self, key: &Self::Key) -> bool {
        let mut map = self.0.write(key);
        map.get_mut(key).map(|v| *v += 1).is_some()
    }
}

#[derive(Clone)]
pub struct DashMapTable<K>(Arc<DashMap<K, u32>>);

impl<K> Collection for DashMapTable<K>
where
    K: Send + Sync + From<u64> + Copy + 'static + std::hash::Hash + Eq + std::fmt::Debug,
{
    type Handle = Self;
    fn with_capacity(capacity: usize) -> Self {
        Self(Arc::new(DashMap::with_capacity(capacity)))
    }

    fn pin(&self) -> Self::Handle {
        self.clone()
    }
}

impl<K> CollectionHandle for DashMapTable<K>
where
    K: Send + Sync + From<u64> + Copy + 'static + std::hash::Hash + Eq + std::fmt::Debug,
{
    type Key = K;

    fn get(&mut self, key: &Self::Key) -> bool {
        self.0.get(key).is_some()
    }

    fn insert(&mut self, key: &Self::Key) -> bool {
        self.0.insert(*key, 0).is_none()
    }

    fn remove(&mut self, key: &Self::Key) -> bool {
        self.0.remove(key).is_some()
    }

    fn update(&mut self, key: &Self::Key) -> bool {
        self.0.get_mut(key).map(|mut v| *v += 1).is_some()
    }
}
