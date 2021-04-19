pub mod adaptive;

#[cfg(test)]
mod tests {
    use super::*;
    use adaptive::AdaptiveFuture;

    #[tokio::test]
    async fn it_works() {
        let thing = AdaptiveFuture::new(|| 1);
        assert_eq!(1, thing.await);
    }

    #[tokio::test]
    #[should_panic]
    async fn it_works_panic() {
        let thing = AdaptiveFuture::new(|| {
            panic!("gus");
            1
        });
        assert_eq!(1, thing.await);
    }
}
