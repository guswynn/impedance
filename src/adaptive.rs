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

use crate::{core::TimedBlockingFuture, token::Token};

/// see [here](https://github.com/guswynn/impedance/blob/main/benches/comparisons.rs#L66-L71`)
/// to get a baseline cost of [spawn_blocking](tokio::task::spawn_blocking) (on your machine)
///
/// Currently this is set to `100_000` nanoseconds. This may change, or need to be made
/// configurable for your usecase.
pub const BLOCKING_CUTOFF_DURATION: Duration = Duration::from_nanos(100000);

#[pin_project]
pub struct AdaptiveFuture<O, F> {
    #[pin]
    inner: TimedBlockingFuture<O, F>,
}

impl<O, F: FnOnce() -> O> AdaptiveFuture<O, F> {
    pub fn new(token: Token, future: F) -> Self {
        AdaptiveFuture {
            inner: TimedBlockingFuture::new(token, BLOCKING_CUTOFF_DURATION, future),
        }
    }
}

impl<O: Send + 'static, F: FnOnce() -> O + Send + 'static> Future for AdaptiveFuture<O, F> {
    type Output = O;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        this.inner.poll(cx)
    }
}
