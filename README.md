# Impedance: Tools to mix blocking and async code


## `cargo +nightly bench`:
```
$ cargo +nightly bench
    Finished bench [optimized] target(s) in 0.62s
     Running target/release/deps/impedance-6efcafb7e2533f63

running 2 tests
test tests::it_works ... ignored
test tests::it_works_panic ... ignored

test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running target/release/deps/spawn-f5f0f45a25148c29

running 4 tests
test no_adaptive_fast   ... bench:       2,936 ns/iter (+/- 222)
test no_adaptive_slow   ... bench: 112,729,812 ns/iter (+/- 5,704,146)
test with_adaptive_fast ... bench:       4,605 ns/iter (+/- 1,575)
test with_adaptive_slow ... bench:  11,895,813 ns/iter (+/- 1,509,361)

test result: ok. 0 passed; 0 failed; 0 ignored; 4 measured; 0 filtered out; finished in 45.16s
```
