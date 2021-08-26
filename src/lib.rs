//! _**Note:** This crate is still in early development and undergoing API changes. Contributions, feature requests, and
//! constructive feedback are warmly welcomed._
//!
//! # sharded &emsp; ![Build] ![Crate]
//!
//! [Build]: https://github.com/nkconnor/sharded/workflows/build/badge.svg
//! [Crate]: https://img.shields.io/crates/v/sharded
//!
//! **Sharded provides safe, fast, and obvious concurrent collections in Rust**. This crate splits the
//! underlying collection into `N shards` each with its own lock. Calling `read(&key)` or `write(&key)`
//! returns a guard for a single shard.
//!
//! ## Features
//!
//! * **Zero unsafe code.** This library uses `#![forbid(unsafe_code)]`. However, it is based on locks
//! and the user must be aware of the potential for deadlocks.
//!
//! * **Zero dependencies.** By default, the library only uses `std`. If you'd like to pull in some community
//! crates such as `parking_lot`, `hashbrown`, `ahash`, etc.. just use add the corresponding feature.
//!
//! * **Tiny footprint.** The core logic is ~100 lines of code. This may build up over time as utilities
//! and ergonomics are added.
//!
//! * ~~**Extremely fast.** This implementation may be a more performant choice for your workload than some
//! of the most popular concurrent hashmaps out there.~~ **??**
//!
//! ### See Also
//!
//! - **[flurry](https://github.com/jonhoo/flurry)** - A port of Java's `java.util.concurrent.ConcurrentHashMap` to Rust. (Also part of a live stream series)
//! - **[dashmap](https://github.com/xacrimon/dashmap)** - Blazing fast concurrent HashMap for Rust.
//! - **[countrie](https://crates.io/crates/contrie)** - A concurrent hash-trie map & set.
//!
//!
//! ## Quick Start
//!
//! ```toml
//! [dependencies]
//!
//! # Optionally use `parking_lot`, `hashbrown`, and `ahash`
//! # by specifing the feature by the same name e.g.
//! sharded = { version = "0.0.1", features = ["fxhash", "parking_lot"] }
//! ```
//! ### Examples
//!
//! **Use a concurrent HashMap**
//!
//! ```
//! use sharded::Map;
//! let concurrent = Map::new();
//! ```
//! ```
//! // or use an existing HashMap,
//! # let users = std::collections::HashMap::new();
//! let users = Shard::from(users);
//! users.insert(32, "Henry");
//! ```
//!
//! ## Acknowledgements
//!
//! Many thanks to
//!
//! - [Reddit community](https://www.reddit.com/r/rust) for a few pointers and
//! some motivation to take this project further.
//!
//! - [Jon Gjengset](https://github.com/jonhoo) for the live streams and utility crates involved
//!
//! - and countless OSS contributors that made this work possible
//!
//! ## License
//!
//! Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
//! 2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
//!
//! Unless you explicitly state otherwise, any contribution intentionally submitted
//! for inclusion in `sharded` by you, as defined in the Apache-2.0 license, shall be
//! dual licensed as above, without any additional terms or conditions.
#![forbid(unsafe_code)]

#[cfg(feature = "fxhash")]
use fxhash_utils::FxHasher as DefaultHasher;

#[cfg(feature = "fxhash")]
use fxhash_utils::FxBuildHasher as DefaultRandomState;

#[cfg(feature = "ahash")]
use ahash_utils::AHasher as DefaultHasher;

#[cfg(feature = "ahash")]
use ahash_utils::RandomState as DefaultRandomState;

#[cfg(feature = "xxhash")]
use xxhash_utils::XxHash64 as DefaultHasher;

#[cfg(feature = "xxhash")]
use xxhash_utils::RandomXxHashBuilder64 as DefaultRandomState;

#[cfg(not(any(feature = "ahash", feature = "fxhash", feature = "xxhash")))]
use std::collections::hash_map::DefaultHasher;

#[cfg(not(any(feature = "ahash", feature = "fxhash", feature = "xxhash")))]
use std::collections::hash_map::RandomState as DefaultRandomState;

#[cfg(feature = "hashbrown")]
use hashbrown_utils::HashMap;

#[cfg(not(feature = "hashbrown"))]
use std::collections::HashMap;

#[cfg(feature = "parking_lot")]
pub type Lock<T> = parking_lot_utils::RwLock<T>;

#[cfg(feature = "parking_lot")]
pub type ReadGuard<'a, T> = parking_lot_utils::RwLockReadGuard<'a, T>;

#[cfg(feature = "parking_lot")]
pub type WriteGuard<'a, T> = parking_lot_utils::RwLockWriteGuard<'a, T>;

#[cfg(not(feature = "parking_lot"))]
pub type Lock<T> = std::sync::RwLock<T>;

#[cfg(not(feature = "parking_lot"))]
type ReadGuard<'a, T> = std::sync::RwLockReadGuard<'a, T>;

#[cfg(not(feature = "parking_lot"))]
type WriteGuard<'a, T> = std::sync::RwLockWriteGuard<'a, T>;

use std::hash::Hash;
use std::hash::Hasher;

pub type RandomState = DefaultRandomState;

/// Number of shards
const DEFAULT_SHARD_COUNT: usize = 128;

/// Get the shared index for the given key
#[inline]
pub(crate) fn index<K: Hash>(k: &K) -> usize {
    let mut s = DefaultHasher::default();
    k.hash(&mut s);
    (s.finish() as usize % DEFAULT_SHARD_COUNT) as usize
}

pub mod evmap;
pub use evmap::EvMap;
pub mod map;
pub use map::Map;

/// The sharded lock collection. This is the main type in the crate. It is more common
/// that you would interface with `Map` and `Set` in the crate root.
pub struct Shard<T> {
    pub(crate) shards: [T; DEFAULT_SHARD_COUNT],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_and_write() {
        let x = Map::new();

        x.write(&"key".to_string())
            .insert("key".to_string(), "value".to_string());

        assert_eq!(
            x.read(&"key".to_string()).get(&"key".to_string()).unwrap(),
            "value"
        );
    }

    #[test]
    fn hold_read_and_write() {
        let map = Map::new();
        let mut write = map.write(&"abc".to_string());
        write.insert("abc".to_string(), "asdf".to_string());

        let _read = map.read(&"asdfas".to_string());
        let _read_too = map.read(&"asdfas".to_string());
        assert!(_read.is_empty());
    }
}
