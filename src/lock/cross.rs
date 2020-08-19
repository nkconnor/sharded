use crate::{index, Lock, Shard};
use crossbeam::sync::{ShardedLock, ShardedLockReadGuard, ShardedLockWriteGuard};
use std::hash::Hash;

impl<T> Lock<T> for ShardedLock<T> {
    #[rustfmt::skip]
    type ReadGuard<'b> where T: 'b = ShardedLockReadGuard<'b, T>;
    #[rustfmt::skip]
    type WriteGuard<'b> where T: 'b = ShardedLockWriteGuard<'b, T>;

    fn new(t: T) -> Self {
        crossbeam::sync::ShardedLock::new(t)
    }

    fn read(&self) -> Self::ReadGuard<'_> {
        self.read().unwrap()
    }

    fn write(&self) -> Self::WriteGuard<'_> {
        self.write().unwrap()
    }
}

impl<T> Shard<ShardedLock<T>> {
    pub fn write<K: Hash>(&self, k: &K) -> ShardedLockWriteGuard<'_, T> {
        let i = index(k);
        self.shards
            .get(i)
            .map(|lock| lock.write().unwrap())
            .unwrap()
    }

    pub fn read<K: Hash>(&self, k: &K) -> ShardedLockReadGuard<'_, T> {
        let i = index(k);
        self.shards.get(i).map(|lock| lock.read().unwrap()).unwrap()
    }
}
