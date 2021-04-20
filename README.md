# Impedance: Tools to mix blocking and async code

See [https://docs.rs/impedance] for more info.

## `cargo +nightly bench`:

(Note that these are doing 10 concurrent operations on an 8-core machine)

```
$ cargo +nightly bench
...
test fast_with_adaptive               ... bench:       4,782 ns/iter (+/- 627)
test fast_with_adaptive_always_inline ... bench:       4,412 ns/iter (+/- 699)
test fast_with_adaptive_always_spawn  ... bench:      55,455 ns/iter (+/- 22,798)
test fast_with_nothing                ... bench:       3,391 ns/iter (+/- 227)
test fast_with_spawn_blocking         ... bench:      51,054 ns/iter (+/- 10,620)
test slow_with_adaptive               ... bench:  12,092,260 ns/iter (+/- 1,572,018)
test slow_with_nothing                ... bench: 122,687,873 ns/iter (+/- 16,353,904)
test slow_with_spawn                  ... bench:  24,730,260 ns/iter (+/- 3,003,759)
test slow_with_spawn_blocking         ... bench:  12,543,033 ns/iter (+/- 2,753,322)
...
```


## License
This project is licensed under either of Apache License, Version 2.0, 
([LICENSE-APACHE](LICENSE-APACHE) or MIT license ([LICENSE-MIT](LICENSE-MIT).
Unless you explicitly state otherwise, any contribution intentionally submitted 
for inclusion in this crate by you, as defined in the Apache-2.0 license, 
shall be dual licensed as above, without any additional terms or conditions.
