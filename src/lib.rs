//! `impedance` is a library that provides utilities to make working with blocking code while in
//! the context of asynchronous code easier. It is named after [this
//! phenomenom](https://en.wikipedia.org/wiki/Impedance_matching).
//!
//! ## Utilities
//!
//! - [`adaptive::AdaptiveFuture`](adaptive::AdaptiveFuture)
//!
//! A wrapper around blocking work that can adaptively move that work to another thread when it is
//! expensive (where expensive == long *wall-time*). It works in tandem with its configuration
//! mechanism [`Token`](adaptive::Token). This can sometimes give us the best of both worlds:
//!
//! ```ignore
//! $ cargo +nightly bench
//! ...
//! test fast_with_adaptive               ... bench:       4,782 ns/iter (+/- 627)
//! test fast_with_adaptive_always_inline ... bench:       4,412 ns/iter (+/- 699)
//! test fast_with_adaptive_always_spawn  ... bench:      55,455 ns/iter (+/- 22,798)
//! test fast_with_nothing                ... bench:       3,391 ns/iter (+/- 227)
//! test fast_with_spawn_blocking         ... bench:      51,054 ns/iter (+/- 10,620)
//! test slow_with_adaptive               ... bench:  12,092,260 ns/iter (+/- 1,572,018)
//! test slow_with_nothing                ... bench: 122,687,873 ns/iter (+/- 16,353,904)
//! test slow_with_spawn                  ... bench:  24,730,260 ns/iter (+/- 3,003,759)
//! test slow_with_spawn_blocking         ... bench:  12,543,033 ns/iter (+/- 2,753,322)
//! ...
//! ```
//! (See [the benchmarks
//! themselves](https://github.com/guswynn/impedance/blob/main/benches/comparisons.rs) for more
//! info)
//!
//! - `buffer_unordered/buffered` helpers (coming hopefully soon)
//! Helpers that avoid pitfalls when using `buffer_unordered`.
//!
//! ## Features
//! This library should be design in a way such that any executor that has a
//! `spawn_blocking` method can be used. However, currently it only implements it for
//! [`tokio`](tokio) which is in its `default_features`.
//!
pub mod adaptive;

#[cfg(test)]
mod tests {
    use super::*;
    use adaptive::{AdaptiveFuture, Token};
    use tokio::runtime::Handle;

    #[tokio::test]
    async fn test_basic() {
        let thing = AdaptiveFuture::new(Token::new(), || 1);
        assert_eq!(1, thing.await);
    }

    #[tokio::test]
    #[should_panic(expected = "Cannot start a runtime from within a runtime")]
    async fn test_nested() {
        let thing = AdaptiveFuture::new(Token::new(), || {
            Handle::current().block_on(async { AdaptiveFuture::new(Token::new(), || 1).await })
        });
        assert_eq!(1, thing.await);
    }

    #[tokio::test]
    #[should_panic(expected = "Cannot start a runtime from within a runtime")]
    async fn test_nested_comparison() {
        let thing = (|| {
            Handle::current().block_on(async { AdaptiveFuture::new(Token::new(), || 1).await })
        })();
        assert_eq!(1, thing);
    }

    #[tokio::test]
    #[should_panic]
    async fn test_panic_adaptive() {
        let thing = AdaptiveFuture::new(Token::new(), || {
            if false {
                1_isize
            } else {
                panic!("gus");
            }
        });
        assert_eq!(1, thing.await);
    }

    #[tokio::test]
    #[should_panic]
    async fn test_panic_spawning() {
        let thing = AdaptiveFuture::new(Token::always_spawn(), || {
            if false {
                1_isize
            } else {
                panic!("gus");
            }
        });
        assert_eq!(1, thing.await);
    }
}
