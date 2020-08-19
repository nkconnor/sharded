# sharded &emsp; ![Build]

[Build]: https://github.com/nkconnor/sharded/workflows/build/badge.svg

_**Warning:** This crate is still in early development and undergoing API changes. Contributions, feature requests, and 
constructive feedback are warmly welcomed._ 

Safe, fast, and obvious concurrent collections for Rust. This crate splits the 
underlying collection into N shards each with its own lock. Calling `read(key)` or `write(key)`
returns a guard for a single shard.

## Features

* **Zero unsafe code.** This library uses #![forbid(unsafe_code)]. There are some limitations with the 
raw locking API that _could cause you to write a bug_, but it should be hard to so!

* **Zero dependencies.** By default, the library only uses `std`. If you'd like to pull in some community
crates such as `parking_lot`, just use the **3rd-party** feature.

* **Tiny footprint.** The core logic is ~100 lines of code. This may build up over time as utility
methods and ergonomics are added.

* **Extremely fast.** This implementation may be a more performant choice for your workload than some
of the most popular concurrent hashmaps out there.

* **Flexible API.**. Bring your own lock or collection types. `sharded::Map` is just a type alias for
`Shard<Lock<Collection<_>>>`. There's support for Sets and Trees, too!

### See Also

- [`dashmap`](https://github.com/xacrimon/dashmap)
- [`flurry`](todo)
- [`countrie`](todo)

## Quick Start 

```toml
[dependencies]

# Optionally use `parking_lot`, `hashbrown`, and `ahash`
# by specifing the feature "3rd-party"

sharded = { version = "0.1.0", features = ["3rd-party"] }
```
### Examples

**Use a concurrent HashMap**

```rust
use sharded::Map;
let concurrent = Map::new()

// or use an existing HashMap,

let users = Shard::from(users);

let guard = users.write(32);
guard.insert(32, user);
```


### Performance Comparison
_**Disclaimer**: I'm no expert in performance testing._ Probably the best you can do is benchmark your application
using the different implementations in the most realistic setting possible.

These measurements were generated using [`jonhoo/bustle`](https://github.com/jonhoo/bustle). To reproduce the charts, 
see the `benchmarks` directory. Work is underway to automate testing on a battery of cloud instance types and parameters. 
Please raise a PR/issue if you have suggestions on how to improve these benchmarks or new 
workloads to try!

#### Average Performance by Implementation

This runs each implementation over the presets in `bustle::Mix` for 5 iterations/random seeds. Lower numbers are better.
All implementations are pretty close but `sharded` wins by some margin until the high thread counts. At `threads=1`, `sharded`
shows a significant advantage.

![Average Performance (read_heavy)](benchmarks/avg_performance_read_heavy.png)


## License

Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in `shard_lock` by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
