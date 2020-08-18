# shard_lock &emsp; ![Build]

[Build]: https://github.com/nkconnor/shard_lock/workflows/build/badge.svg

A generic sharded locking mechanism for hash based collections to speed up concurrent reads/writes. `Shard::new` splits
the underlying collection into N shards each with its own lock. Calling `read(key)` or `write(key)`
returns a guard for only a single shard. The underlying locks should be generic, so you can use
it with any `Mutex` or `RwLock` in `std::sync` or `parking_lot`.

In a probably wrong and unscientific test of concurrent readers/single writer, 
`shard_lock` is **100x-∞∞∞**(deadlocks..) faster than [`dashmap`](https://github.com/xacrimon/dashmap), and
**13x** faster than a single `parking_lot::RwLock`. Carrying `Shard<RwLock<T>>` is possibly more obvious
and simpler than other approaches. The library has a very small footprint at ~100 loc and optionally no
dependencies.

`shard_lock` is flexible enough to shard any hash based collection such as `HashMap`, `HashSet`, `BTreeMap`, and `BTreeSet`.

_**Warning:** shard_lock is in early development and unsuitable for production. The API is undergoing changes and is not dependable._

**Feedback and Contributions appreciated!**


## Getting Started

```toml
[dependencies]

# Specify support for external locks using feature keys:
#  - parking_lot (RwLock, Mutex, ..)
shard_lock = { version = "0.0.1", features = ["parking_lot"] }
```

## Examples

```rust
// or Shard::<()>::new(HashMap::new()); not sure how to get rid of the turbofish..
let users = shard!(HashMap::new()); 

let guard = users.write(32);
guard.insert(32, user);
```

## License

Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in `shard_lock` by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
