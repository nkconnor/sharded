use crate::collection::Collection;
use crate::lock::Lock;
use crate::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

// Global shard count for collections
// TODO configurable via construction
const SHARD_COUNT: usize = 128;

/// Teases out the sharding key for example
/// from an IntoIterator value.
pub trait ExtractShardKey<K: Hash> {
    fn key(&self) -> &K;
}

// Takes key from map iter values
impl<K: Hash, V> ExtractShardKey<K> for (K, V) {
    fn key(&self) -> &K {
        &self.0
    }
}

pub(crate) fn index<K: Hash>(k: &K) -> usize {
    let mut s = DefaultHasher::default();
    k.hash(&mut s);
    (s.finish() as usize % SHARD_COUNT) as usize
}

/// The sharded lock collection. This is the main data type in the crate. See also the type aliases
/// `Map`, `Set`, and so on.
///
/// # Examples
///
/// ```ignore
/// use sharded::Shard;
///
/// let users = Shard::from(HashMap::new());
///
/// let guard = users.read("uid-31356");
///
/// guard.get("uid-31356");
/// ```
pub struct Shard<T> {
    pub(crate) shards: Vec<T>,
}

impl<K: Hash> Shard<K> {
    /// Create a new shard from an existing collection
    pub fn from<V, U, L>(inner: U) -> Shard<L>
    where
        V: ExtractShardKey<K>,
        U: Collection<K, V>,
        L: Lock<U>,
    {
        let mut shards = vec![U::with_capacity(inner.len() / SHARD_COUNT); SHARD_COUNT];

        inner.into_iter().for_each(|item| {
            // for each item, push it to the appropriate shard
            let i = index(item.key());
            if let Some(shard) = shards.get_mut(i) {
                shard.insert(item)
            } else {
                panic!(
                    "We just initialized shards to `SHARD_COUNT` and hash % `SHARD_COUNT`
                    should be bounded"
                );
            }
        });

        let shards = shards.into_iter().map(|shard| L::new(shard)).collect();

        Shard { shards }
    }
}
