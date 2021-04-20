# Impedance: Tools to mix blocking and async code


## `cargo +nightly bench`:
```
$ cargo +nightly bench
...
test fast_with_adaptive       ... bench:       4,617 ns/iter (+/- 238)
test fast_with_nothing        ... bench:       2,800 ns/iter (+/- 195)
test fast_with_spawn_blocking ... bench:      53,920 ns/iter (+/- 17,066)
test slow_with_adaptive       ... bench:  11,916,003 ns/iter (+/- 1,802,266)
test slow_with_nothing        ... bench: 123,418,131 ns/iter (+/- 18,761,112)
test slow_with_spawn_blocking ... bench:  11,356,039 ns/iter (+/- 1,396,545)
...
```
