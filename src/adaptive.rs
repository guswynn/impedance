use pin_project::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::oneshot::{channel,  Receiver};
use tokio::task::{JoinHandle, JoinError};

#[pin_project]
pub struct AdaptiveFuture<O, F: FnOnce() -> O> {
    fut: Option<F>,
    inner: Option<JoinHandle<O>>,
    wakeup: Option<Receiver<()>>,
}

impl<O, F: FnOnce() -> O> AdaptiveFuture<O, F> {
    pub fn new(future: F) -> Self {
        AdaptiveFuture {
            fut: Some(future),
            inner: None,
            wakeup: None,
        }
    }
}

impl<O: Send + 'static, F: FnOnce() -> O + Send + 'static> Future for AdaptiveFuture<O, F>
{
    type Output = O;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        println!("GUS");

        match this.fut.take() {
            Some(f) => {
                let (tx, mut rx) = channel();

                let jh = tokio::spawn(
                    async move {
                        let ret = f();
                        tx.send(());
                        ret});
                // Register the waker (lol)
                Pin::new(&mut rx).poll(cx);
                *this.wakeup = Some(rx);
                *this.inner = Some(jh);
                Poll::Pending
            }
            None => {
                let jh = this.inner.as_mut().expect("should have something here");

                match Pin::new(jh).poll(cx) {
                    Poll::Ready(Ok(val)) => Poll::Ready(val),
                    Poll::Ready(Err(e)) => match e.try_into_panic() {
                        Ok(panic) => {
                            std::panic::resume_unwind(panic);
                        }
                        Err(_) => {
                            // Task is shutting down so lets just pend
                            Poll::Pending
                        }
                    },
                    _ => Poll::Pending,
                }
            }
        }
    }
}
