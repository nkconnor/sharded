use crate::shard::index;
use crate::{Lock, Shard};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::hash::Hash;

impl<T> Lock<T> for RwLock<T> {
    #[rustfmt::skip]
    type ReadGuard<'b> where T: 'b = RwLockReadGuard<'b, T>;
    #[rustfmt::skip]
    type WriteGuard<'b> where T: 'b = RwLockWriteGuard<'b, T>;

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

impl<T> Shard<RwLock<T>> {
    pub fn write<K: Hash>(&self, k: &K) -> RwLockWriteGuard<'_, T> {
        let i = index(k);
        self.shards.get(i).map(|lock| lock.write()).unwrap()
    }

    pub fn read<K: Hash>(&self, k: &K) -> RwLockReadGuard<'_, T> {
        let i = index(k);
        self.shards.get(i).map(|lock| lock.read()).unwrap()
    }
}
