//! Error type for `tui-lab-assertions`.

/// Errors raised while evaluating assertions.
#[derive(Debug)]
pub enum AssertionError {
    /// Free-form failure with condition context attached upstream.
    Message(String),
}

impl std::fmt::Display for AssertionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Message(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for AssertionError {}

/// Crate result alias.
pub type Result<T> = std::result::Result<T, AssertionError>;
