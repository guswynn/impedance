pub mod adaptive;

#[cfg(test)]
mod tests {
    use super::*;
    use adaptive::AdaptiveFuture;

    #[tokio::test]
    async fn it_works() {
        let thing = AdaptiveFuture::new(|| {
            1});
        assert_eq!(1, thing.await);
    }
}
