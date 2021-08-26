use crate::*;
use std::convert::TryInto;
use std::fmt::Debug;

/// Sharded lock-based concurrent map using the crate default lock and map implementations.
pub type Map<K, V> = Shard<Lock<HashMap<K, V, RandomState>>>;

impl<K, V> From<HashMap<K, V, RandomState>> for Map<K, V>
where
    K: Hash + Eq + Clone + Debug,
    V: Clone + Debug,
{
    fn from(inner: HashMap<K, V, RandomState>) -> Self {
        let capacity = inner.len() / DEFAULT_SHARD_COUNT;
        let hasher = RandomState::default();
        let mut shards =
            vec![HashMap::with_capacity_and_hasher(capacity, hasher); DEFAULT_SHARD_COUNT];

        inner.into_iter().for_each(|(key, value)| {
            shards.get_mut(index(&key))
                .map(|shard| {
                    shard.insert(key, value);
                })
            .unwrap_or_else(|| {
                panic!(
                    "We just initialized shards to `DEFAULT_SHARD_COUNT` and hash % `DEFAULT_SHARD_COUNT` should be bounded")
            });
        });

        let shards = shards
            .into_iter()
            .map(Lock::new)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        Shard { shards }
    }
}

impl<K, V> Map<K, V> {
    /// Create an empty sharded map
    pub fn new() -> Map<K, V>
    where
        K: Hash + Eq + Clone + Debug,
        V: Clone + Debug,
    {
        Shard::from(HashMap::<K, V, RandomState>::with_hasher(
            RandomState::default(),
        ))
    }

    /// Create a sharded map with the provided capacity
    pub fn with_capacity(capacity: usize) -> Map<K, V>
    where
        K: Hash + Eq + Clone + Debug,
        V: Clone + Debug,
    {
        Shard::from(HashMap::<K, V, RandomState>::with_capacity_and_hasher(
            capacity,
            RandomState::default(),
        ))
    }

    /// Insert a key value pair into the Map. Returns the existing
    /// value at the provided key if there was one.
    #[inline]
    pub fn insert(&self, k: K, v: V) -> Option<V>
    where
        K: Hash + Eq,
    {
        self.write(&k).insert(k, v)
    }

    /// Get a read guard to the shard corresponding to the provided key
    ///
    /// **Panics** if the shard lock is poisoned
    #[inline]
    pub fn read(&self, k: &K) -> ReadGuard<'_, HashMap<K, V, RandomState>>
    where
        K: Hash,
    {
        let i = index(&k);

        self.shards
            .get(i)
            .map(|lock| {
                #[cfg(feature = "parking_lot")]
                {
                    lock.read()
                }
                #[cfg(not(feature = "parking_lot"))]
                {
                    lock.read().unwrap()
                }
            })
            .unwrap_or_else(|| panic!("index out of bounds"))
    }

    /// Get a write guard to the shard corresponding to the provided key
    ///
    /// **Panics** if the shard lock is poisoned
    #[inline]
    pub fn write(&self, k: &K) -> WriteGuard<'_, HashMap<K, V, RandomState>>
    where
        K: Hash,
    {
        let i = index(&k);

        self.shards
            .get(i)
            .map(|lock| {
                #[cfg(feature = "parking_lot")]
                {
                    lock.write()
                }
                #[cfg(not(feature = "parking_lot"))]
                {
                    lock.write().unwrap()
                }
            })
            .unwrap_or_else(|| panic!("index out of bounds"))
    }
}
