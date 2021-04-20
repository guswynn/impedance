pub mod adaptive;
mod core;
pub mod token;

#[cfg(test)]
mod tests {
    use super::*;
    use adaptive::AdaptiveFuture;
    use token::Token;
    use tokio::runtime::Handle;

    #[tokio::test]
    async fn basic() {
        let thing = AdaptiveFuture::new(Token::new(), || 1);
        assert_eq!(1, thing.await);
    }

    #[tokio::test]
    #[should_panic(expected = "Cannot start a runtime from within a runtime")]
    async fn nested() {
        let thing = AdaptiveFuture::new(Token::new(), || {
            Handle::current().block_on(async { AdaptiveFuture::new(Token::new(), || 1).await })
        });
        assert_eq!(1, thing.await);
    }

    #[tokio::test]
    #[should_panic(expected = "Cannot start a runtime from within a runtime")]
    async fn nested_comparison() {
        let thing = (|| {
            Handle::current().block_on(async { AdaptiveFuture::new(Token::new(), || 1).await })
        })();
        assert_eq!(1, thing);
    }

    #[tokio::test]
    #[should_panic]
    async fn panic() {
        let thing = AdaptiveFuture::new(Token::new(), || {
            if false {
                1_isize
            } else {
                panic!("gus");
            }
        });
        assert_eq!(1, thing.await);
    }
}
