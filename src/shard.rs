use crate::collection::Collection;
use crate::lock::Lock;
use crate::DefaultHasher;
use crate::HashMap;
use crate::ShardLock;
use std::hash::Hash;
use std::hash::Hasher;

use std::ops::Deref;

struct GetGuard<'a, K, V, U, L>
where
    L: Lock<U>,
    U: 'a,
{
    key: &'a K,
    value: &'a V,
    guard: L::ReadGuard<'a>,
}

impl<'a, K, V, U, L> Deref for GetGuard<'a, K, V, U, L>
where
    L: Lock<U>,
{
    type Target = V;

    fn deref(&self) -> &V {
        self.value
    }
}
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

// Identity fn
impl<K: Hash> ExtractShardKey<K> for K {
    fn key(&self) -> &K {
        &self
    }
}

pub(crate) fn index<K: Hash>(k: &K, shard_count: usize) -> usize {
    let mut s = DefaultHasher::default();
    k.hash(&mut s);
    (s.finish() as usize % shard_count) as usize
}

/// The sharded lock collection. This is the main data type in the crate. See also the type aliases
/// `Map`, `Set`, and so on.
///
/// # Examples
///
/// Shard a `std::collections::HashMap` using `std::sync::RwLock`
///
/// ```
/// use std::collections::HashMap;
/// use std::sync::RwLock;
/// use sharded::{ShardLock, Shard};
/// # #[derive(Clone)]
/// # struct User {}
///
/// let users = Shard::<RwLock<_>>::from(HashMap::<&str, User>::new());
///
/// let guard = users.read(&"uid-31356");
/// let user: Option<&User> = guard.get(&"uid-31356");
/// ```
pub struct Shard<T> {
    pub(crate) shards: Vec<T>,
    count: usize,
}

// Convenience methods for Collection = HashMap.
// I think this may not work out later on though because
// there will be a name clash and T is not constrained at the top
impl<T> Shard<T> {
    pub fn insert<K, V>(&self, k: K, v: V) -> Option<V>
    where
        K: Hash + Eq + Clone,
        V: Clone,
        T: Lock<HashMap<K, V>>,
    {
        self.write(&k).insert(k, v)
    }

    //argh its so close
    //    pub fn get<'a, K, V, S: 'a>(
    //        &'a self,
    //        key: &'a K,
    //    ) -> Option<GetGuard<'a, K, V, HashMap<K, V, S>, T>>
    //    where
    //        K: Hash + Eq + Clone,
    //        V: Clone,
    //        S: BuildHasher + Clone + Default,
    //        T: Lock<HashMap<K, V, S>>,
    //    {
    //        let guard: T::ReadGuard<'a> = self.read(key);
    //
    //        if let Some(inner) = guard.get(&key) {
    //            Some(GetGuard {
    //                key: &key,
    //                value: inner,
    //                guard,
    //            })
    //        } else {
    //            None
    //        }
    //    }
}

impl<T> Shard<T> {
    /// Create a new shard from an existing collection
    pub fn from<K, V, U>(inner: U) -> Self
    where
        K: Hash,
        V: ExtractShardKey<K>,
        U: Collection<K, V>,
        T: Lock<U>,
    {
        let count = SHARD_COUNT;
        let mut shards = vec![U::with_capacity(inner.len() / count); count];

        inner.into_iter().for_each(|item| {
            // for each item, push it to the appropriate shard
            let i = index(item.key(), count);
            if let Some(shard) = shards.get_mut(i) {
                shard.insert(item);
            } else {
                panic!(
                    "We just initialized shards to `SHARD_COUNT` and hash % `SHARD_COUNT`
                    should be bounded"
                );
            }
        });

        let shards = shards.into_iter().map(|shard| T::new(shard)).collect();

        Shard { shards, count }
    }

    pub(crate) fn index<K, V, U>(&self, key: &K) -> usize
    where
        V: ExtractShardKey<K>,
        K: Hash,
        U: Collection<K, V>,
        T: Lock<U>,
    {
        index(&key, self.count)
    }
}

// WIP, possibly blocked on GAT bug
//pub mod ev {
//    use crate::lock::Lock;
//    use crate::{Collection, ExtractShardKey, HashMap, RwLock, Shard, ShardLock};
//    use std::hash::Hash;
//    use std::sync::mpsc::{channel, Sender};
//    use std::sync::{Arc, Mutex};
//    use std::thread::JoinHandle;
//
//    /// An eventually consistent sharded collection.
//    pub struct Ev<K, V, U, L>
//    where
//        K: Hash,
//        V: ExtractShardKey<K>,
//        U: Collection<K, V>,
//        L: Lock<U>,
//    {
//        pub shard: Arc<Shard<L>>,
//        sender: Mutex<Sender<V>>,
//        receiver: JoinHandle<()>,
//        phantom: std::marker::PhantomData<(U, K)>,
//    }
//
//    impl<K, V, U, L> Ev<K, V, U, L>
//    where
//        K: Hash,
//        V: ExtractShardKey<K> + Send + 'static,
//        U: Collection<K, V>,
//        L: Lock<U> + Sync + Send + Sized + 'static,
//    {
//        pub fn from(inner: U) -> Self {
//            let shard: Shard<L> = Shard::from::<K, V, U>(inner);
//            let shard = Arc::new(shard);
//            let writer = Arc::clone(&shard);
//
//            let (tx, rx) = channel::<V>();
//
//            let handle = std::thread::spawn(move || {
//                //
//                loop {
//                    if let Ok(v) = rx.recv() {
//                        let mut part = writer.write(v.key());
//                        part.insert(v);
//                    }
//                }
//            });
//
//            Self {
//                shard,
//                sender: Mutex::new(tx),
//                receiver: handle,
//                phantom: std::marker::PhantomData::default(),
//            }
//        }
//
//        pub fn insert(&self, v: V) {
//            let lock = self.sender.lock().unwrap();
//            lock.send(v).unwrap();
//        }
//    }
//
//    fn break_static_f(key: String) {
//        let ev: Ev<
//            String,
//            (String, String),
//            HashMap<String, String>,
//            RwLock<HashMap<String, String>>,
//        > = Ev::from(HashMap::new());
//        let mut guard = ev.shard.write(&key);
//        guard.insert("asdfa".to_string(), "asdfa".to_string());
//    }
//}
//#[cfg(test)]
//mod tests {
//    use super::ev::*;
//    use crate::*;
//
//    #[test]
//    fn break_static_f() {
//        let key = "asdfas".to_string();
//
//        let ev: Ev<
//            String,
//            (String, String),
//            HashMap<String, String>,
//            RwLock<HashMap<String, String>>,
//        > = Ev::from(HashMap::new());
//        let mut guard = ev.shard.write(&key);
//        guard.insert("asdfa".to_string(), "asdfa".to_string());
//    }
//
//    //https://github.com/rust-lang/rust/issues/68648
//    //fn break_static_f() {
//    //    let ev: EvShard<RwLock<HashMap<String, String>>, _> = EvShard::from(HashMap::new());
//    //    let mut guard = ev.shard.write(&"asdfa".to_string());
//    //    guard.insert("asdfa".to_string(), "asdfa".to_string());
//    //}
//}
