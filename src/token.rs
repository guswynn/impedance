use std::sync::atomic::{AtomicUsize, Ordering};

// TODO(guswynn): seqcst?
static CURRENT: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
enum TokenType {
    AdhocAdaptive(usize),
    AlwaysInline,
    AlwaysSpawn,
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct Token(TokenType);

impl Token {
    pub fn new() -> Self {
        Token(TokenType::AdhocAdaptive(
            CURRENT.fetch_add(1, Ordering::SeqCst),
        ))
    }
}
