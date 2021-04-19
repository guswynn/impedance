use std::sync::atomic::{AtomicUsize, Ordering};

// TODO(guswynn): seqcst?
static CURRENT: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct Token(usize);

impl Token {
    pub fn new() -> Self {
        Token(CURRENT.fetch_add(1, Ordering::SeqCst))
    }
}
