//! _**Note:** This crate is still in early development and undergoing API changes. Contributions, feature requests, and
//! constructive feedback are warmly welcomed._
//!
//! # sharded &emsp; ![Build] ![Crate]
//!
//! [Build]: https://github.com/nkconnor/sharded/workflows/build/badge.svg
//! [Crate]: https://img.shields.io/crates/v/sharded
//!
//! **Sharded provides safe, fast, and obvious concurrent collections in Rust**. This crate splits the
//! underlying collection into `N shards` each with its own lock. Calling `read(key)` or `write(key)`
//! returns a guard for a single shard.
//!
//! ## Features
//!
//! * **Zero unsafe code.** This library uses `#![forbid(unsafe_code)]`. There are some limitations with the
//! raw locking API that _could cause you to write a bug_, but it should be hard to so!
//!
//! * **Zero dependencies.** By default, the library only uses `std`. If you'd like to pull in some community
//! crates such as `parking_lot`, just use the **3rd-party** feature.
//!
//! * **Tiny footprint.** The core logic is ~100 lines of code. This may build up over time as utilities
//! and ergonomics are added.
//!
//! * ~~**Extremely fast.** This implementation may be a more performant choice for your workload than some
//! of the most popular concurrent hashmaps out there.~~ **??**
//!
//! * **Flexible API.**. Bring your own lock or collection types. `sharded::Map` is just a type alias for
//! `Shard<Lock<Collection<_>>>`. There's support for Sets and Trees, too!
//!
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
//! # by specifing the feature "3rd-party"
//! sharded = { version = "0.0.1", features = ["3rd-party"] }
//! ```
//! ### Examples
//!
//! **Use a concurrent HashMap**
//!
//! ```rust
//! use sharded::Map;
//! let concurrent = Map::new()
//!
//! // or use an existing HashMap,
//!
//! let users = Shard::from(users);
//!
//! let guard = users.write(32);
//! guard.insert(32, user);
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
#![allow(dead_code)]
#![allow(unused_macros)]
#![allow(incomplete_features)]
#![feature(generic_associated_types)]
#![feature(in_band_lifetimes)]

#[cfg(feature = "hash-ahash")]
use ahash::AHasher as DefaultHasher;

#[cfg(not(feature = "hash-ahash"))]
use std::collections::hash_map::DefaultHasher;

#[cfg(feature = "map-hashbrown")]
use hashbrown::HashMap;

#[cfg(feature = "map-hashbrown")]
use hashbrown::HashSet;

#[cfg(not(feature = "map-hashbrown"))]
use std::collections::HashMap;

#[cfg(not(feature = "map-hashbrown"))]
use std::collections::HashSet;

use std::hash::Hash;

mod lock;
pub use lock::Lock;
pub use lock::RwLock;
pub use lock::ShardLock;

mod collection;
pub use collection::Collection;

mod shard;
pub use shard::ExtractShardKey;
pub use shard::Shard;

/// Sharded lock-based concurrent map using the crate default lock and map implementations.
pub type Map<K, V> = Shard<RwLock<HashMap<K, V>>>;

/// Sharded lock-based concurrent set using the crate default lock and set implementations.
pub type Set<K> = Shard<RwLock<HashSet<K>>>;

impl<K: Hash + Eq + Clone, V: Clone> Map<K, V> {
    pub fn new() -> Self {
        Shard::from(HashMap::new())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Shard::from(HashMap::with_capacity(capacity))
    }
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
