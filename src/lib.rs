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
//! mechanism [`Token`](adaptive::Token).
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
