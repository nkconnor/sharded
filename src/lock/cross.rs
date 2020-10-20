use crate::Lock;
use crossbeam_utils::sync::{ShardedLock, ShardedLockReadGuard, ShardedLockWriteGuard};

impl<T> Lock<T> for ShardedLock<T> {
    #[rustfmt::skip]
    type ReadGuard<'b> where T: 'b = ShardedLockReadGuard<'b, T>;
    #[rustfmt::skip]
    type WriteGuard<'b> where T: 'b = ShardedLockWriteGuard<'b, T>;

    fn new(t: T) -> Self {
        ShardedLock::new(t)
    }

    fn read(&self) -> Self::ReadGuard<'_> {
        self.read().unwrap()
    }

    fn write(&self) -> Self::WriteGuard<'_> {
        self.write().unwrap()
    }
}
