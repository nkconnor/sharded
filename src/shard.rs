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

//impl<K: Hash, V, U, L, S> Ops<K> for S
//where
//    V: ExtractShardKey<K>,
//    U: Collection<K, V>,
//    L: Lock<U>,
//    S: ShardOn<K, V = V, U = U, L = L>,
//{
//    type V = V;
//    type U = U;
//    type L = L;
//    fn write<'a>(&'a self, k: &K) -> <<Self as Ops<K>>::L as Lock<Self::U>>::WriteGuard<'a> {
//        let i = index(k);
//        if let Some(lock) = self.shards().get(i) {
//            lock.write()
//        } else {
//            panic!("asdfa");
//        }
//    }
//}

//mod ev {
//    use crate::lock::Lock;
//    use crate::*;
//    use std::hash::Hash;
//    use std::sync::mpsc::{channel, Receiver, Sender};
//    use std::sync::Mutex;
//    use std::thread::JoinHandle;
//
//    /// An eventually consistent sharded collection.
//    pub struct EvShard<T, V> {
//        pub(crate) shards: Vec<T>,
//        sender: Mutex<Sender<V>>,
//        receiver: JoinHandle<()>,
//    }
//
//    impl<K: Hash, V> EvShard<K, V>
//    where
//        V: ExtractShardKey<K>,
//    {
//        pub fn from<U, L>(inner: U) -> EvShard<L, V>
//        where
//            U: Collection<K, V>,
//            L: Lock<U>,
//        {
//            let shard = Shard::from::<V, U, L>(inner);
//
//            let (tx, rx) = channel::<V>();
//
//            let handle = std::thread::spawn(|| loop {
//                if let Ok(v) = rx.recv() {
//                    let shard: U = *shard.write(v.key());
//                    shard.insert(v);
//                }
//            });
//
//            EvShard {
//                shards: shard.shards,
//                sender: Mutex::new(tx),
//                receiver: handle,
//            }
//        }
//    }
//}
