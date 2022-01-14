//! _**Note: This crate is still in early development and undergoing API changes.** Contributions, feature requests, and
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
//! * **Zero unsafe code.** This library uses `#![forbid(unsafe_code)]` and was motivated by
//! the complexity and amount of memory errors present in many alternatives.
//!
//! * **Tiny footprint.** The core logic is ~100 lines of code. This may build up over time as utilities
//! and ergonomics are added. By default, the library only uses `std` and `hashbrown`. If you'd like to pull in some community
//! crates such as `parking_lot`, `ahash`, etc.. just use add the corresponding feature.
//!
//! * **Really fast.** This implementation may be a more performant choice than some
//! of the most popular concurrent hashmaps out there. Try it on your workload and let us know.
//!
//! ### See Also
//!
//! - **[countrie](https://crates.io/crates/contrie)** - A concurrent hash-trie map & set.
//! - **[dashmap](https://github.com/xacrimon/dashmap)** - Blazing fast concurrent HashMap for Rust.
//! - **[flurry](https://github.com/jonhoo/flurry)** - A port of Java's `java.util.concurrent.ConcurrentHashMap` to Rust. (Also part of a live stream series)
//!
//! ## Quick Start
//!
//! ```toml
//! [dependencies]
//! # Optionally use `parking_lot`, `ahash`, `fxhash`, and `xxhash`
//! # by specifing the feature by the same name e.g.
//! sharded = { version = "0.1.0", features = ["fxhash", "parking_lot"] }
//! ```
//! ### Examples
//!
//! **Insert a key value pair**
//!
//! ```
//! # use sharded::Map;
//! let users = Map::new();
//! users.insert(32, "Henry");
//! ```
//!
//! **Access a storage shard**
//!
//! `Map` provides `read` and `write` which give access to the underlying
//! storage (which is built using `hashbrown::raw`). Both methods return a tuple of `(Key,
//! Guard<Shard>)`
//!
//! ```
//! # use sharded::Map;
//! # let users = Map::new();
//! # users.insert(32, "Henry");
//! let (key, shard) = users.read(&32);
//! assert_eq!(shard.get(key), Some(&"Henry"));
//! ```
//!
//! **Determine if a storage shard is locked**
//!
//! `try_read` and `try_write` are available for avoiding blocks or in situations that could
//! deadlock
//!
//! ```
//! # use sharded::Map;
//! # let users = Map::new();
//! # users.insert(32, "Henry");
//! # struct WouldBlock;
//! match users.try_read(&32) {
//!     Some((key, mut shard)) => Ok(shard.get(key)),
//!     None => Err(WouldBlock)
//! };
//! ```
//!
//! ## Performance Comparison
//!
//! These measurements were generated using [`jonhoo/bustle`](https://github.com/jonhoo/bustle). To reproduce the charts,
//! see the `benchmarks` directory. Benchmarks can be misleading. It is recommended to benchmark using a real application
//! workload.
//!
//! ### Average Performance by Implementation
//!
//! This ran each implementation over the presets in [`bustle::Mix`](https://docs.rs/bustle/0.4.2/bustle/struct.Mix.html) for 5
//! iterations / random seeds using a Intel® Core™ i9-9820X. Lower numbers are better. Approaches using a single `std::sync` Lock and `chashmap` were discarded for clarity (they are
//! a lot slower). If you know why `chashmap` is so slow in this test, please help here.
//!
//! ![Read Heavy Performance)](benchmarks/avg_performance_read_heavy.png)
//! ![Write Heavy Performance)](benchmarks/avg_performance_write_heavy.png)
//! ![Update Heavy Performance)](benchmarks/avg_performance_update_heavy.png)
//! ![Uniform Performance)](benchmarks/avg_performance_uniform.png)
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

//#[cfg(feature = "fxhash")]
//use fxhash_utils::FxHasher as DefaultHasher;

#[cfg(feature = "fxhash")]
use fxhash_utils::FxBuildHasher as DefaultRandomState;

//#[cfg(feature = "ahash")]
//use ahash_utils::AHasher as DefaultHasher;

#[cfg(feature = "ahash")]
use ahash_utils::RandomState as DefaultRandomState;

//#[cfg(feature = "xxhash")]
//use xxhash_utils::XxHash64 as DefaultHasher;

#[cfg(feature = "xxhash")]
use xxhash_utils::RandomXxHashBuilder64 as DefaultRandomState;

#[cfg(feature = "seahash")]
mod seahash {
    use std::hash::BuildHasher;

    use seahash_utils::SeaHasher;

    #[derive(Default, Clone)]
    pub struct DefaultState;

    impl BuildHasher for DefaultState {
        type Hasher = SeaHasher;

        fn build_hasher(&self) -> Self::Hasher {
            SeaHasher::new()
        }
    }
}

#[cfg(feature = "seahash")]
use seahash::DefaultState as DefaultRandomState;

//#[cfg(not(any(feature = "ahash", feature = "fxhash", feature = "xxhash")))]
//use std::collections::hash_map::DefaultHasher;

#[cfg(not(any(
    feature = "ahash",
    feature = "fxhash",
    feature = "xxhash",
    feature = "seahash"
)))]
use std::collections::hash_map::RandomState as DefaultRandomState;

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

pub type RandomState = DefaultRandomState;

/// Number of shards
const DEFAULT_SHARD_COUNT: u64 = 128;

pub mod map;
pub use map::Map;
