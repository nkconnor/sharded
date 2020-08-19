#[cfg(feature = "lock-crossbeam")]
mod cross;

#[cfg(feature = "lock-parking-lot")]
mod parking;

use crate::{index, Lock, Shard};
use std::hash::Hash;

#[cfg(feature = "lock-parking-lot")]
pub type RwLock<T> = parking_lot::RwLock<T>;

#[cfg(feature = "lock-crossbeam")]
pub type RwLock<T> = crossbeam::sync::ShardedLock<T>;

#[cfg(not(any(feature = "lock-parking-lot", feature = "lock-crossbeam")))]
pub type RwLock<T> = std::sync::RwLock<T>;

use std::sync::{RwLock as StdRwLock, RwLockReadGuard, RwLockWriteGuard};

impl<T> Lock<T> for StdRwLock<T> {
    #[rustfmt::skip]
    type ReadGuard<'b> where T: 'b = RwLockReadGuard<'b, T>;
    #[rustfmt::skip]
    type WriteGuard<'b> where T: 'b = RwLockWriteGuard<'b, T>;

    fn new(t: T) -> Self {
        StdRwLock::new(t)
    }

    fn read(&self) -> Self::ReadGuard<'_> {
        self.read().unwrap()
    }

    fn write(&self) -> Self::WriteGuard<'_> {
        self.write().unwrap()
    }
}

impl<T> Shard<StdRwLock<T>> {
    pub fn write<K: Hash>(&self, k: &K) -> RwLockWriteGuard<'_, T> {
        let i = index(k);
        self.shards
            .get(i)
            .map(|lock| lock.write().unwrap())
            .unwrap()
    }

    pub fn read<K: Hash>(&self, k: &K) -> RwLockReadGuard<'_, T> {
        let i = index(k);
        self.shards.get(i).map(|lock| lock.read().unwrap()).unwrap()
    }
}
