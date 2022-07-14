use crate::*;
use hashbrown::raw::{RawIntoIter, RawTable};

use std::borrow::Borrow;
use std::convert::TryInto;
use std::hash::{BuildHasher, Hash, Hasher};
use std::{fmt, fmt::Debug};

#[cfg(not(feature = "parking_lot"))]
use std::sync::TryLockError;

/// Ensures that a single closure type across uses of this which, in turn prevents multiple
/// instances of any functions like RawTable::reserve from being generated
#[inline]
fn equivalent_key<K, V>(k: &K) -> impl Fn(&(K, V)) -> bool + '_
where
    K: ?Sized + Eq,
{
    move |x| k.eq(x.0.borrow())
}

#[inline]
fn make_hash<K, S>(hash_builder: &S, val: &K) -> u64
where
    K: Hash,
    S: BuildHasher,
{
    let mut state = hash_builder.build_hasher();
    val.hash(&mut state);
    state.finish()
}

/// Ensures that a single closure type across uses of this which, in turn prevents multiple
/// instances of any functions like RawTable::reserve from being generated
#[inline]
fn make_hasher<K, V, S>(hash_builder: &S) -> impl Fn(&(K, V)) -> u64 + '_
where
    K: Hash,
    S: BuildHasher,
{
    move |val| make_hash::<K, S>(hash_builder, &val.0)
}

/// A shard key
pub trait Key<K> {
    fn hash(&self) -> u64;
    fn key(&self) -> &K;
}

/// Can do reads only
pub struct ReadKey<'a, K>(u64, &'a K);
impl<'a, K> Key<K> for ReadKey<'a, K> {
    #[inline]
    fn hash(&self) -> u64 {
        self.0
    }
    #[inline]
    fn key(&self) -> &K {
        self.1
    }
}

/// Can do reads and writes
pub struct WriteKey<K>(u64, K);
impl<K> WriteKey<K> {
    pub fn into_inner(self) -> K {
        self.1
    }
}

impl<K> Clone for WriteKey<K>
where
    K: Clone,
{
    fn clone(&self) -> Self {
        WriteKey(self.hash(), self.key().clone())
    }
}

impl<K> Key<K> for WriteKey<K> {
    #[inline]
    fn hash(&self) -> u64 {
        self.0
    }

    #[inline]
    fn key(&self) -> &K {
        &self.1
    }
}

/// Sharded, lock-based hash map using the crate default lock
pub struct Map<K, V, S = RandomState> {
    hash_builder: S,
    shards: [Lock<Shard<K, V>>; DEFAULT_SHARD_COUNT as usize],
}

/// A single shard in the map
#[derive(Clone)]
pub struct Shard<K, V, S = RandomState> {
    hash_builder: S,
    inner: RawTable<(K, V)>,
}

impl<K, V> Debug for Shard<K, V>
where
    K: Debug,
    V: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("shard").finish()
    }
}

impl<K, V> Shard<K, V> {
    /// Number of items in the shard
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Is `len == 0`
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.len() == 0
    }

    /// Remove the key, returning the value at that position if it existed
    #[inline]
    pub fn remove(&mut self, key: WriteKey<K>) -> Option<V>
    where
        K: Hash + Eq,
    {
        #[allow(clippy::manual_map)] // reduce compiler IR, I think!
        match self
            .inner
            .remove_entry(key.hash(), equivalent_key(key.key()))
        {
            Some((_, v)) => Some(v),
            None => None,
        }
    }

    /// Get mutable value for the provided key
    #[inline]
    pub fn get_mut<Q: Key<K>>(&mut self, key: Q) -> Option<&mut V>
    where
        K: Hash + Eq,
    {
        match self.inner.get_mut(key.hash(), equivalent_key(key.key())) {
            Some(&mut (_, ref mut v)) => Some(v),
            None => None,
        }
    }

    /// Insert the key value pair
    #[inline]
    pub fn insert(&mut self, key: WriteKey<K>, v: V) -> Option<V>
    where
        K: Hash + Eq,
    {
        if let Some((_, item)) = self.inner.get_mut(key.hash(), equivalent_key(key.key())) {
            Some(std::mem::replace(item, v))
        } else {
            self.inner.insert(
                key.hash(),
                (key.into_inner(), v),
                make_hasher::<K, V, _>(&self.hash_builder),
            );
            None
        }
    }

    /// Get the value for the key if it exists
    #[inline]
    pub fn get<Q: Key<K>>(&self, key: Q) -> Option<&V>
    where
        K: Hash + Eq,
    {
        match self.inner.get(key.hash(), equivalent_key(key.key())) {
            Some(&(_, ref v)) => Some(v),
            None => None,
        }
    }
}

impl<K, V> Map<K, V> {
    /// Create a new map with the provided capacity. This will distribute the capacity
    /// evenly among all the shards (well.. see below)
    pub fn with_capacity(capacity: usize) -> Self
    where
        K: Debug,
        V: Debug,
    {
        Self::with_capacity_and_hasher(capacity, RandomState::default())
    }

    /// Create a new map with the provided capacity and hash_builder. This will distribute the capacity
    /// evenly among all the shards
    pub fn with_capacity_and_hasher(capacity: usize, hash_builder: RandomState) -> Self
    where
        K: Debug,
        V: Debug,
    {
        let capacity = (capacity as u64 + DEFAULT_SHARD_COUNT - 1) / DEFAULT_SHARD_COUNT;

        let shards = std::iter::repeat(|| RawTable::with_capacity(capacity as usize))
            .map(|f| f())
            .take(DEFAULT_SHARD_COUNT as usize)
            .map(|inner| {
                Lock::new(Shard {
                    inner,
                    hash_builder: hash_builder.clone(),
                })
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        Self {
            hash_builder,
            shards,
        }
    }

    /// Get a read guard to the shard corresponding to the provided key
    ///
    /// **Panics** if the shard lock is poisoned
    #[inline]
    pub fn read<'a>(&'a self, key: &'a K) -> (ReadKey<'a, K>, ReadGuard<'a, Shard<K, V>>)
    where
        K: Hash + Eq,
    {
        let hash = make_hash::<K, _>(&self.hash_builder, key);

        let i = hash % DEFAULT_SHARD_COUNT;

        let shard = match self.shards.get(i as usize) {
            Some(lock) => {
                #[cfg(feature = "parking_lot")]
                {
                    lock.read()
                }
                #[cfg(not(feature = "parking_lot"))]
                {
                    lock.read().unwrap()
                }
            }
            None => panic!("index out of bounds"),
        };

        (ReadKey(hash, key), shard)
    }

    /// Attempt to retrieve a read guard for the shard corresponding to the provided key. If
    /// a writer currently holds the lock, this will return `None`
    ///
    /// **Panics** if the shard lock is poisoned
    #[allow(clippy::type_complexity)]
    #[inline]
    pub fn try_read<'a>(
        &'a self,
        key: &'a K,
    ) -> Option<(ReadKey<'a, K>, ReadGuard<'a, Shard<K, V>>)>
    where
        K: Hash + Eq,
    {
        let hash = make_hash::<K, _>(&self.hash_builder, key);

        let i = hash % DEFAULT_SHARD_COUNT;

        let shard = match self.shards.get(i as usize) {
            Some(lock) => {
                #[cfg(feature = "parking_lot")]
                {
                    match lock.try_read() {
                        Some(v) => v,
                        None => return None,
                    }
                }
                #[cfg(not(feature = "parking_lot"))]
                {
                    match lock.try_read() {
                        Ok(v) => v,
                        Err(TryLockError::Poisoned(_)) => {
                            panic!("Tried to read on a poisoned lock")
                        }
                        Err(TryLockError::WouldBlock) => return None,
                    }
                }
            }
            None => panic!("index out of bounds"),
        };

        Some((ReadKey(hash, key), shard))
    }

    /// Does the map contain the provided key
    #[inline]
    pub fn contains<'a>(&'a self, key: &'a K) -> bool
    where
        K: Eq + Hash,
    {
        let (key, shard) = self.read(key);
        shard.get(key).is_some()
    }

    /// Number of elements in the map
    #[inline]
    pub fn len(&self) -> usize {
        #[cfg(feature = "parking_lot")]
        return self.shards.iter().map(|x| x.read().len()).sum();

        #[cfg(not(feature = "parking_lot"))]
        return self.shards.iter().map(|x| x.read().unwrap().len()).sum();
    }

    /// Is `len == 0`
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a cloned value corresponding to the provided key
    #[inline]
    pub fn get_owned<'a>(&'a self, key: &'a K) -> Option<V>
    where
        K: Eq + Hash,
        V: Clone,
    {
        let (key, shard) = self.read(key);
        shard.get(key).cloned()
    }

    /// Get a read guard to the shard corresponding to the provided key
    ///
    /// **Panics** if the shard lock is poisoned
    #[inline]
    pub fn write(&self, key: K) -> (WriteKey<K>, WriteGuard<Shard<K, V>>)
    where
        K: Hash + Eq,
    {
        let hash = make_hash::<K, _>(&self.hash_builder, &key);

        let i = hash % DEFAULT_SHARD_COUNT;

        let shard = match self.shards.get(i as usize) {
            Some(lock) => {
                #[cfg(feature = "parking_lot")]
                {
                    lock.write()
                }
                #[cfg(not(feature = "parking_lot"))]
                {
                    lock.write().unwrap()
                }
            }
            None => panic!("index out of bounds"),
        };

        (WriteKey(hash, key), shard)
    }

    /// Insert a key value pair into the Map. Returns the existing
    /// value at the provided key if there was one.
    #[inline]
    pub fn insert(&self, k: K, v: V) -> Option<V>
    where
        K: Hash + Eq,
    {
        let (key, mut shard) = self.write(k);
        shard.insert(key, v)
    }

    /// Remove using the provided key. Returns the existing value, if any.
    pub fn remove(&self, k: K) -> Option<V>
    where
        K: Hash + Eq,
    {
        let (key, mut shard) = self.write(k);
        shard.remove(key)
    }

    /// Create an empty sharded map
    pub fn new() -> Map<K, V>
    where
        K: Debug,
        V: Debug,
    {
        Self::with_capacity(0)
    }

    /// Creates a consuming iterator, that is, one that moves each key-value
    /// pair out of the map in arbitrary order. The map cannot be used after
    /// calling this. Yields the values of the map.
    pub fn into_values(self) -> IntoValues<K, V> {
        IntoValues {
            iter: self.into_iter(),
        }
    }
}

impl<K: Debug, V: Debug> Default for Map<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: 'static, V: 'static> IntoIterator for Map<K, V> {
    type IntoIter = IntoIter<K, V>;

    type Item = (K, V);

    /// Creates a consuming iterator, that is, one that moves each key-value
    /// pair out of the map in arbitrary order. The map cannot be used after
    /// calling this.
    fn into_iter(self) -> IntoIter<K, V> {
        let shards: Vec<_> = self.shards.into();

        #[cfg(feature = "parking_lot")]
        let mut shards: Vec<Shard<K, V>> = shards.into_iter().map(|s| s.into_inner()).collect();

        #[cfg(not(feature = "parking_lot"))]
        let mut shards: Vec<Shard<K, V>> = shards
            .into_iter()
            .map(|s| s.into_inner().unwrap())
            .collect();

        IntoIter {
            iter: shards.pop().unwrap().inner.into_iter(),
            shards,
        }
    }
}

pub struct IntoIter<K: 'static, V: 'static> {
    iter: RawIntoIter<(K, V)>,
    shards: Vec<Shard<K, V>>,
}

pub struct IntoValues<K: 'static, V: 'static> {
    iter: IntoIter<K, V>,
}

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);

    #[inline]
    fn next(&mut self) -> Option<(K, V)> {
        match self.iter.next() {
            Some(item) => Some(item),
            None => match self.shards.pop() {
                Some(s) => {
                    self.iter = s.inner.into_iter();
                    self.next()
                }
                None => None,
            },
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.iter.size_hint().0, None)
    }
}

impl<K, V> Iterator for IntoValues<K, V> {
    type Item = V;

    #[inline]
    fn next(&mut self) -> Option<V> {
        self.iter.next().map(|(_, v)| v)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.iter.size_hint().0, None)
    }
}

//scratch work
//use std::iter::Extend;
//
//impl<K, V> Extend<(K, V)> for Map<K, V>
//where
//    K: Hash + Eq + Send + Sync + 'static,
//    V: Send + Sync + 'static,
//{
//    fn extend<T>(&mut self, iter: T)
//    where
//        T: IntoIterator<Item = (K, V)>,
//    {
//        let iter = iter.into_iter();
//        // iter.size_hint()
//
//        let t_handles = Vec::with_capacity(DEFAULT_SHARD_COUNT as usize);
//        let txs = Vec::with_capacity(DEFAULT_SHARD_COUNT as usize);
//
//        for i in 0..DEFAULT_SHARD_COUNT {
//            let shard = self.shards[i as usize].write().unwrap();
//            let shard = std::sync::Arc::new(shard);
//            // ^ need crossbeam probably
//            let (tx, rx) = std::sync::mpsc::channel();
//            txs.push(tx);
//
//            std::thread::spawn(move || {
//                for (key, value) in rx {
//                    shard.insert(key, value);
//                }
//            });
//        }
//
//        let (rx, tx) = std::sync::mpsc::channel();
//    }
//}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_guard_holds_the_lock_and_read_guard_blocks() {
        let map = Map::with_capacity(1);
        let (key, mut guard) = map.write("");
        guard.insert(key, "value");

        // since the guard is still held, this should block
        assert!(map.try_read(&"").is_none())
    }

    #[test]
    fn read_and_write_with_lock_held() {
        let map = Map::with_capacity(1);
        let (key, mut guard) = map.write("");
        guard.insert(key.clone(), "value");

        assert_eq!(guard.get(key), Some(&"value"))
    }

    #[test]
    fn into_iter_yields_one_expected_value() {
        let map = Map::with_capacity(1);
        map.insert("k1", "v1");
        assert_eq!(
            map.into_iter().collect::<Vec<_>>().pop().unwrap(),
            ("k1", "v1")
        );

        let map = Map::with_capacity(1);
        map.insert("k1", "v1");
        assert_eq!(map.into_values().collect::<Vec<_>>().pop().unwrap(), "v1");
    }

    #[test]
    fn into_iter_has_4_iters() {
        let map = Map::with_capacity(4);
        map.insert("k1", "v1");
        map.insert("k2", "v2");
        map.insert("k3", "v3");
        map.insert("k4", "v4");
        assert_eq!(map.into_iter().map(|_| 1).sum::<u32>(), 4);

        let map = Map::with_capacity(4);
        map.insert("k1", "v1");
        map.insert("k2", "v2");
        map.insert("k3", "v3");
        map.insert("k4", "v4");
        assert_eq!(map.into_values().map(|_| 1).sum::<u32>(), 4);
    }
}
