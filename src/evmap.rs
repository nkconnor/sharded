//use crate::*;
//use std::fmt::Debug;
//use std::sync::{mpsc, mpsc::SyncSender, Arc};
//use std::{thread, thread::JoinHandle};
//
///// Sharded lock-based concurrent map using the crate default lock and map implementations.
//pub struct EvMap<K, V> {
//    shard: Arc<Shard<Lock<HashMap<K, V, RandomState>>>>,
//    sender: Arc<SyncSender<(K, V)>>,
//    _event_loop: Arc<JoinHandle<()>>,
//}
//
//impl<K, V> Clone for EvMap<K, V> {
//    fn clone(&self) -> Self {
//        EvMap {
//            shard: self.shard.clone(),
//            sender: self.sender.clone(),
//            _event_loop: self._event_loop.clone(),
//        }
//    }
//}
//
//impl<K, V> From<HashMap<K, V, RandomState>> for EvMap<K, V>
//where
//    K: Hash + Eq + Clone + Debug + Sync + Send + 'static,
//    V: Clone + Debug + Sync + Send + 'static,
//{
//    fn from(inner: HashMap<K, V, RandomState>) -> Self {
//        let shard = Map::from(inner);
//        Self::new_with_shard(shard)
//    }
//}
//
//impl<K, V> EvMap<K, V> {
//    fn new_with_shard(shard: Shard<Lock<HashMap<K, V, RandomState>>>) -> Self
//    where
//        K: Hash + Eq + Clone + Debug + Sync + Send + 'static,
//        V: Clone + Debug + Sync + Send + 'static,
//    {
//        let shard = Arc::new(shard);
//        let (tx, rx) = mpsc::sync_channel(DEFAULT_SHARD_COUNT.pow(2));
//
//        let write_shard = shard.clone();
//
//        let _event_loop = thread::spawn(move || {
//            while let Ok((k, v)) = rx.recv() {
//                write_shard.insert(k, v);
//            }
//        });
//
//        Self {
//            shard,
//            sender: Arc::new(tx),
//            _event_loop: Arc::new(_event_loop),
//        }
//    }
//
//    /// Create an empty sharded eventually consistent map
//    pub fn new() -> Self
//    where
//        K: Hash + Eq + Clone + Debug + Sync + Send + 'static,
//        V: Clone + Debug + Sync + Send + 'static,
//    {
//        Self::new_with_shard(Map::new())
//    }
//
//    /// Create an empty sharded eventually consistent map with
//    /// the provided capacity
//    pub fn with_capacity(capacity: usize) -> Self
//    where
//        K: Hash + Eq + Clone + Debug + Sync + Send + 'static,
//        V: Clone + Debug + Sync + Send + 'static,
//    {
//        Self::new_with_shard(Map::with_capacity(capacity))
//    }
//
//    /// Insert a key value pair into the Map. Returns the existing
//    /// value at the provided key if there was one.
//    #[inline]
//    pub fn insert(&self, k: K, v: V) -> bool
//    where
//        K: Hash + Eq,
//    {
//        let existed = { self.read(&k).contains_key(&k) };
//        self.sender.send((k, v)).unwrap();
//        existed
//    }
//
//    /// Get a read guard to the shard corresponding to the provided key
//    ///
//    /// **Panics** if the shard lock is poisoned
//    #[inline]
//    pub fn read(&self, k: &K) -> ReadGuard<'_, HashMap<K, V, RandomState>>
//    where
//        K: Hash,
//    {
//        self.shard.read(&k)
//    }
//
//    /// Get a write guard to the shard corresponding to the provided key
//    ///
//    /// **Panics** if the shard lock is poisoned
//    #[inline]
//    pub fn write(&self, k: &K) -> WriteGuard<'_, HashMap<K, V, RandomState>>
//    where
//        K: Hash,
//    {
//        self.shard.write(&k)
//    }
//}
