use crate::shard::index;
use crate::{Lock, Shard};
use parking_lot_utils::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::hash::Hash;

impl<T> Lock<T> for RwLock<T> {
    #[rustfmt::skip]
    type ReadGuard<'b> where T: 'b = RwLockReadGuard<'b, T>;
    #[rustfmt::skip]
    type WriteGuard<'b> where T: 'b = RwLockWriteGuard<'b, T>;

    fn new(t: T) -> Self {
        RwLock::new(t)
    }

    #[inline]
    fn write(&self) -> Self::WriteGuard<'_> {
        self.write()
    }

    #[inline]
    fn read(&self) -> Self::ReadGuard<'_> {
        self.read()
    }
}