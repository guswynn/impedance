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
