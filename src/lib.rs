pub mod adaptive;
pub mod token;

#[cfg(test)]
mod tests {
    use super::*;
    use adaptive::AdaptiveFuture;
    use token::Token;

    #[tokio::test]
    async fn it_works() {
        let thing = AdaptiveFuture::new(Token::new(), || 1);
        assert_eq!(1, thing.await);
    }

    #[tokio::test]
    #[should_panic]
    async fn it_works_panic() {
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
