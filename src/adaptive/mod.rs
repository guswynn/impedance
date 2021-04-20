//! Adaptively schedule blocking work
//!
//! The `adaptive` module defines types that help you schedule *blocking work* (namely, not
//! `async`) adaptively. If the work is cheap, we save on the overhead of things like
//! [spawn_blocking](tokio::task::spawn_blocking) by skipping moving work to a thread
//! and instead running it inline in the [poll](std::future::Future::poll) implementation.
//!
//! The most important type in this module is [AdaptiveFuture], which wraps an [FnOnce]
//! representing the blocking work you want to perform.
//!
//! ## Usage
//! ```
//! use impedance::adaptive::{AdaptiveFuture, Token};
//! use once_cell::sync::Lazy;
//!
//! static REQUEST_TOKEN: Lazy<Token> = Lazy::new(||
//!     Token::new()
//! );
//!
//! fn deserialize(s: &str) -> i32 {
//!     // could be expensive to deserialize!
//!     1
//! }
//!
//! async fn send_request() -> String {
//!     // We are usually doing io here
//!     "gus".to_string()
//! }
//!
//! async fn make_request() -> i32 {
//!     let response = send_request().await;
//!     AdaptiveFuture::new(*REQUEST_TOKEN, move || deserialize(&response)).await
//! }
//! ```
//!
//! ## Scheduling Scheme
//! `AdaptiveFuture` decides when to inline work based on the last *wall-time* of the work
//! it has performed. The granularity of this *wall-time* is based on the *[Token]* passed
//! into the constructor. This allows to user to have fine-grained control based on their
//! knowledge of how the *possibly-expensvie* cpu work they are guarding with an `AdaptiveFuture`
//! will perform, in different parts of their program.
//!
//!
//! To see more information about how to construct `Token`'s and various options, see [Token].
//! The above example shows the common-case default of using a
//! `static` *unique* `Token` configured to use the default cutoff time ([BLOCKING_CUTOFF_DURATION])
//!
//! More complex scheduling schemes may be available in the future.
use pin_project::pin_project;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

mod token;
pub use token::Token;
mod core;
use self::core::TimedBlockingFuture;

/// see [here](https://github.com/guswynn/impedance/blob/main/benches/comparisons.rs#L66-L71`)
/// to get a baseline cost of [spawn_blocking](tokio::task::spawn_blocking) (on your machine)
///
/// Currently this is set to `100_000` nanoseconds. This may change, or need to be made
/// configurable for your usecase.
pub const BLOCKING_CUTOFF_DURATION: Duration = Duration::from_nanos(100000);

/// A [Future] representing *blocking work*
///
/// It either
/// 1. Runs work inline in its [poll](std::future::Future::poll) implementation
/// 2. Schedules the work on another thread using [spawn_blocking](tokio::task::spawn_blocking)
///
/// see *[the module documentation](self)* for usage examples.
#[pin_project]
pub struct AdaptiveFuture<O, F> {
    #[pin]
    inner: TimedBlockingFuture<O, F>,
}

impl<O, F: FnOnce() -> O> AdaptiveFuture<O, F> {
    /// Create a new `AdaptiveFuture` that will adaptively schedule blocking work
    /// inline or in thread ([spawn_blocking](tokio::task::spawn_blocking)) associated
    /// the [Token](token::Token)
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
