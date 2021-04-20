use std::sync::atomic::{AtomicUsize, Ordering};

// TODO(guswynn): Do I need seqcst?
static CURRENT: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub(crate) enum TokenType {
    AdhocAdaptive(usize),
    AlwaysInline,
    AlwaysSpawn,
}

/// `Token` is a type that is used to associate blocking work with itself and configure
/// [AdaptiveFuture](super::AdaptiveFuture)'s.
///
/// A token to configure and track *wall-times* for work in [AdaptiveFuture](super::AdaptiveFuture)'s
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct Token(pub(crate) TokenType);

impl Token {
    /// Create a new *unique* `Token`, either to use a `static` or a one-off.
    /// This `Token` is configured to start out work as
    /// *inline in the poll implementation*, and to adaptively switch to spawning.
    pub fn new() -> Self {
        Token(TokenType::AdhocAdaptive(
            CURRENT.fetch_add(1, Ordering::SeqCst),
        ))
    }

    /// Create a new `Token` that tells an [AdaptiveFuture](super::AdaptiveFuture) to
    /// always inline its work into its poll implementation.
    pub fn always_inline() -> Self {
        Token(TokenType::AlwaysInline)
    }

    /// Create a new `Token` that tells an [AdaptiveFuture](super::AdaptiveFuture) to
    /// always move its work onto a thread.
    pub fn always_spawn() -> Self {
        Token(TokenType::AlwaysSpawn)
    }
}
