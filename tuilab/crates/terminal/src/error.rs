//! Error type for `tui-lab-terminal`.

/// Errors raised by the terminal emulator adapter.
#[derive(Debug)]
pub enum TerminalError {
    /// Free-form failure with grid context attached upstream.
    Message(String),
}

impl std::fmt::Display for TerminalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Message(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for TerminalError {}

/// Crate result alias.
pub type Result<T> = std::result::Result<T, TerminalError>;
