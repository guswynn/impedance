use once_cell::sync::Lazy;
use parking_lot::Mutex;
use pin_project::pin_project;
use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::{
    sync::oneshot::{channel, Receiver},
    task::JoinHandle,
};

use crate::token::Token;

static DATA: Lazy<Mutex<HashMap<Token, AdaptiveState>>> = Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Clone, Copy)]
enum AdaptiveState {
    Inline,
    Spawn,
}

impl Default for AdaptiveState {
    fn default() -> Self {
        AdaptiveState::Inline
    }
}

#[pin_project]
pub struct AdaptiveFuture<O, F: FnOnce() -> O> {
    fut: Option<F>,
    token: Token,
    inner: Option<JoinHandle<O>>,
    wakeup: Option<Receiver<()>>,
}

impl<O, F: FnOnce() -> O> AdaptiveFuture<O, F> {
    pub fn new(token: Token, future: F) -> Self {
        AdaptiveFuture {
            fut: Some(future),
            token,
            inner: None,
            wakeup: None,
        }
    }
}

impl<O: Send + 'static, F: FnOnce() -> O + Send + 'static> Future for AdaptiveFuture<O, F> {
    type Output = O;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        println!("GUS");

        match this.fut.take() {
            Some(f) => {
                match (*DATA.lock()).entry(*this.token).or_default() {
                    AdaptiveState::Inline => Poll::Ready(f()),
                    AdaptiveState::Spawn => {
                        let (tx, mut rx) = channel();

                        let jh = tokio::task::spawn_blocking(move || {
                            let ret = f();
                            let _ = tx.send(());
                            ret
                        });
                        // Register the waker
                        // Unused is okay as any response is simply to
                        // register a waker
                        let _ = Pin::new(&mut rx).poll(cx);
                        *this.wakeup = Some(rx);
                        *this.inner = Some(jh);
                        Poll::Pending
                    }
                }
            }
            None => {
                let jh = this.inner.as_mut().expect("re-polled a Ready Future");

                // Always re-register the waker
                // Unused is okay as any response is simply to
                // register a waker
                let _ = Pin::new(
                    &mut this
                        .wakeup
                        .as_mut()
                        .expect("wakeup should be set at the same time as inner"),
                )
                .poll(cx);

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
