### Average Performance by Implementation

This ran each implementation over the presets in [`bustle::Mix`](https://docs.rs/bustle/0.4.1/bustle/struct.Mix.html) for 5 
iterations/random seeds. Lower numbers are better. Approaches using a single `std::sync` Lock and `chashmap` were discarded for clarity (they are
a lot slower). If you know why `chashmap` is so slow in this test, please help here.

We used a Intel® Core™ i9-9820X for this test. Work is underway to automate the benchmarks across
cloud based instance types for a number of parameters.

![Read Heavy Performance](avg_performance_read_heavy.png)

![Write Heavy Performance](avg_performance_write_heavy.png)

![Update Heavy Performance](avg_performance_update_heavy.png)

** Note `Flurry` is a partial run in the uniform workload due to OOM. `src/adapters.rs` uses the `flurry:HashMapRef`
which doesn't clear garbage between runs.
![Uniform Performance](avg_performance_uniform.png)

