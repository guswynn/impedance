use once_cell::sync::Lazy;
use parking_lot::Mutex;
use pin_project::pin_project;
use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};
// TODO(azw): support more executors
#[cfg(feature = "tokio")]
use tokio::{
    sync::oneshot::{channel, Receiver},
    task::{spawn_blocking, JoinHandle},
};

use crate::token::Token;

const BLOCKING_DURATION: Duration = Duration::from_millis(5);
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

fn run_and_bench<O, F: FnOnce() -> O>(token: Token, f: F) -> O {
    let now = Instant::now();
    let ret = f();

    if now.elapsed() > BLOCKING_DURATION {
        DATA.lock().insert(token, AdaptiveState::Spawn);
    } else {
        DATA.lock().insert(token, AdaptiveState::Inline);
    }
    ret
}

impl<O: Send + 'static, F: FnOnce() -> O + Send + 'static> Future for AdaptiveFuture<O, F> {
    type Output = O;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        println!("GUS");

        match this.fut.take() {
            Some(f) => {
                // Need to drop the lock before entering the run_and_bench section

                let state = { *(*DATA.lock()).entry(*this.token).or_default() };
                match state {
                    AdaptiveState::Inline => Poll::Ready(run_and_bench(*this.token, f)),
                    AdaptiveState::Spawn => {
                        let (tx, mut rx) = channel();

                        let token = *this.token;
                        let jh = spawn_blocking(move || {
                            let ret = run_and_bench(token, f);
                            let _ = tx.send(());
                            ret
                        });
                        // Register the waker
                        // This is a new Receiver, so we can drop the result
                        // after the first poll
                        let _ = Pin::new(&mut rx).poll(cx);
                        *this.wakeup = Some(rx);
                        *this.inner = Some(jh);
                        Poll::Pending
                    }
                }
            }
            None => {
                let jh = this.inner.as_mut().expect("re-polled a Ready Future");

                // Re-register the waker if its still possible
                match this.wakeup.as_mut() {
                    Some(rx) => match Pin::new(rx).poll(cx) {
                        Poll::Ready(_) => {
                            *this.wakeup = None;
                        }
                        _ => {}
                    },
                    _ => {}
                }

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
