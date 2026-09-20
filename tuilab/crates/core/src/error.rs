//! Error type for `tui-lab-core`.
//!
//! Libraries return `Result<_, CoreError>`; binaries map to exit codes.

/// Errors raised by session orchestration.
#[derive(Debug)]
pub enum CoreError {
    /// The child could not be spawned (CLI maps to exit 3).
    Launch(String),
    /// PTY I/O failed mid-run: read, write, resize, wait (CLI maps to exit 3).
    Pty(String),
    /// A bounded operation (e.g. graceful close) timed out (CLI maps to exit 4).
    Timeout(String),
    /// Free-form failure with session/step context attached upstream.
    Message(String),
}

impl std::fmt::Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Launch(msg) => write!(f, "launch failed: {msg}"),
            Self::Pty(msg) => write!(f, "pty failed: {msg}"),
            Self::Timeout(msg) => write!(f, "timed out: {msg}"),
            Self::Message(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for CoreError {}

/// Crate result alias.
pub type Result<T> = std::result::Result<T, CoreError>;
