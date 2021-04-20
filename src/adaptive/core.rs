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

use super::token::{Token, TokenType};

static TIMINGS: Lazy<Mutex<HashMap<Token, AdaptiveState>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

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
pub struct TimedBlockingFuture<O, F> {
    fut: Option<F>,
    token: Token,
    cutoff: Duration,
    inner: Option<JoinHandle<O>>,
    wakeup: Option<Receiver<()>>,
}

impl<O, F: FnOnce() -> O> TimedBlockingFuture<O, F> {
    pub fn new(token: Token, cutoff: Duration, future: F) -> Self {
        TimedBlockingFuture {
            fut: Some(future),
            cutoff,
            token,
            inner: None,
            wakeup: None,
        }
    }
}

fn track_and_run<O, F: FnOnce() -> O>(token: Token, cutoff: Duration, f: F) -> O {
    let now = Instant::now();
    let ret = f();

    if now.elapsed() > cutoff {
        TIMINGS.lock().insert(token, AdaptiveState::Spawn);
    } else {
        TIMINGS.lock().insert(token, AdaptiveState::Inline);
    }
    ret
}

impl<O: Send + 'static, F: FnOnce() -> O + Send + 'static> Future for TimedBlockingFuture<O, F> {
    type Output = O;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        match this.fut.take() {
            Some(f) => {
                let state = match this.token.0 {
                    TokenType::AlwaysInline => AdaptiveState::Inline,
                    TokenType::AlwaysSpawn => AdaptiveState::Spawn,
                    // Need to drop the lock before entering the `track_and_run` section
                    _ => *(*TIMINGS.lock()).entry(*this.token).or_default(),
                };

                match state {
                    AdaptiveState::Inline => {
                        println!("inline chosen");
                        // Just run it inline
                        Poll::Ready(track_and_run(*this.token, *this.cutoff, f))
                    }
                    AdaptiveState::Spawn => {
                        // Spawn the blocking task
                        let (tx, mut rx) = channel();
                        let jh = {
                            let token = *this.token;
                            let cutoff = *this.cutoff;
                            spawn_blocking(move || {
                                let ret = track_and_run(token, cutoff, f);
                                let _ = tx.send(());
                                ret
                            })
                        };

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
                println!("gus");
                let jh = this.inner.as_mut().expect("re-polled a Ready Future");

                // Re-register the waker if its still possible
                // TODO(guswynn): is this needed?
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
                            // Task is shutdown so lets just pend
                            Poll::Pending
                        }
                    },
                    _ => Poll::Pending,
                }
            }
        }
    }
}
