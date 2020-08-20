#[cfg(feature = "crossbeam")]
mod cross;

#[cfg(feature = "parking_lot")]
mod parking;

use crate::Shard;
use crate::*;
use std::hash::Hash;

#[cfg(feature = "parking_lot")]
pub type RwLock<T> = parking_lot_utils::RwLock<T>;

#[cfg(feature = "crossbeam")]
pub type RwLock<T> = crossbeam_utils::sync::ShardedLock<T>;

#[cfg(not(any(feature = "parking_lot", feature = "crossbeam")))]
pub type RwLock<T> = std::sync::RwLock<T>;

use std::sync::{RwLock as StdRwLock, RwLockReadGuard, RwLockWriteGuard};

/// Generic locking implementation.
pub trait Lock<T> {
    #[rustfmt::skip]
    type ReadGuard<'a>: std::ops::Deref<Target=T> where T: 'a + std::ops::Deref<Target=T>;
    #[rustfmt::skip]
    type WriteGuard<'a>: std::ops::Deref<Target=T> + std::ops::DerefMut<Target=T> where T: 'a;

    fn new(t: T) -> Self;

    fn write(&self) -> Self::WriteGuard<'_>;

    fn read(&self) -> Self::ReadGuard<'_>;
}

pub trait ShardLock<K: Hash, V, U, L>
where
    V: ExtractShardKey<K>,
    U: Collection<K, V>,
    L: Lock<U>,
{
    fn shards<'a>(&'a self) -> &'a [L];
    fn write(&self, k: &K) -> L::WriteGuard<'_>;
    fn read(&self, k: &K) -> L::ReadGuard<'_>;
}

impl<K: Hash, V, U, L> ShardLock<K, V, U, L> for Shard<L>
where
    V: ExtractShardKey<K>,
    U: Collection<K, V>,
    L: Lock<U>,
{
    #[inline]
    fn shards<'a>(&'a self) -> &'a [L] {
        &self.shards
    }

    #[inline]
    fn write(&self, k: &K) -> L::WriteGuard<'_> {
        let i = Shard::<L>::index(&self, &k);

        if let Some(lock) = self.shards.get(i) {
            lock.write()
        } else {
            panic!("index out of bounds")
        }
    }

    #[inline]
    fn read(&self, k: &K) -> L::ReadGuard<'_> {
        let i = Shard::<L>::index(&self, &k);

        if let Some(lock) = self.shards.get(i) {
            lock.read()
        } else {
            panic!("index out of bounds")
        }
    }
}

impl<T> Lock<T> for StdRwLock<T> {
    #[rustfmt::skip]
    type ReadGuard<'a> where T: 'a = RwLockReadGuard<'a, T>;
    #[rustfmt::skip]
    type WriteGuard<'a> where T: 'a = RwLockWriteGuard<'a, T>;

    fn new(t: T) -> Self {
        StdRwLock::new(t)
    }

    #[inline]
    fn read(&self) -> Self::ReadGuard<'_> {
        self.read().unwrap()
    }

    #[inline]
    fn write(&self) -> Self::WriteGuard<'_> {
        self.write().unwrap()
    }
}
