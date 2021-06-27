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
#[cfg(feature = "async-std-experimental")]
use async_std::task::{spawn_blocking, JoinHandle};
#[cfg(feature = "async-std-experimental")]
use futures::channel::oneshot::{channel, Receiver};
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

        loop {
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
                            // Just run it inline
                            return Poll::Ready(track_and_run(*this.token, *this.cutoff, f));
                        }
                        AdaptiveState::Spawn => {
                            // Spawn the blocking task
                            let (tx, rx) = channel();
                            let jh = {
                                let token = *this.token;
                                let cutoff = *this.cutoff;
                                spawn_blocking(move || {
                                    let ret = track_and_run(token, cutoff, f);
                                    // Panic's cause tx to be dropped which will wake the
                                    // Reciever
                                    let _ = tx.send(());
                                    ret
                                })
                            };

                            // Store the reciever to poll later
                            *this.wakeup = Some(rx);
                            // Store the handle to the spawned blocking task, to join later
                            *this.inner = Some(jh);

                            // TODO(guswynn): This is hacky, I should just make a waker myself,
                            // but making sure I wake when tx is dropped is non-trivial
                            //
                            // TODO(guswynn): figure out how to control this in cfg(test):
                            // std::thread::sleep(Duration::from_secs(1));
                            //
                            // Polling the Reciever registers the task's waker
                            // If the blocking thread is working fast, we may immediately
                            // see that we are ready. We aren't just wrapping around as an
                            // optimization: We must read the value off the JoinHandle
                            // as we no longer have any way to recieve wakeups.
                            match Pin::new(this.wakeup.as_mut().unwrap()).poll(cx) {
                                Poll::Ready(_) => {
                                    *this.wakeup = None;
                                    continue;
                                }
                                Poll::Pending => {}
                            }

                            return Poll::Pending;
                        }
                    }
                }
                None => {
                    let jh = this.inner.as_mut().expect("re-polled a Ready Future");

                    // Re-register the waker if its still possible
                    // If we got a ready, we need to busy poll so we can get join handle
                    // value signalled by this channel being ready. We MUST busy poll;
                    // its not an optimization: once we consume our last possible wakeup,
                    // we need to get the value out of the join handle
                    //
                    // TODO(guswynn): catch_unwind in the closure yourself and send the value
                    // from the JoinHandle?
                    let busy_poll = match this.wakeup.as_mut() {
                        Some(rx) => match Pin::new(rx).poll(cx) {
                            Poll::Ready(_) => {
                                // Like above, we don't care about the value,
                                // as joining the handle below is what we want.
                                *this.wakeup = None;
                                true
                            }
                            Poll::Pending => {
                                // Explicit false, as our waker it correctly registered in the
                                // Reciever
                                false
                            }
                        },
                        None => {
                            // an unset reciever means we need to continue to loop
                            // until the join handle gives us the value
                            true
                        }
                    };

                    match Pin::new(jh).poll(cx) {
                        #[cfg(feature = "async-std-experimental")]
                        Poll::Ready(val) => return Poll::Ready(val),
                        #[cfg(feature = "tokio")]
                        Poll::Ready(Ok(val)) => return Poll::Ready(val),
                        #[cfg(feature = "tokio")]
                        Poll::Ready(Err(e)) => match e.try_into_panic() {
                            Ok(panic) => {
                                std::panic::resume_unwind(panic);
                            }
                            Err(_) => {
                                // Task is shutdown so we just pend:
                                // We never abort the sub-task ourselves, so something
                                // else is shutting everything else down, and the task
                                // polling us will likely be shutting down as well
                                // TODO(guswynn): figure out a way to test this
                                return Poll::Pending;
                            }
                        },
                        Poll::Pending if busy_poll => continue,
                        Poll::Pending => return Poll::Pending,
                    }
                }
            }
        }
    }
}
