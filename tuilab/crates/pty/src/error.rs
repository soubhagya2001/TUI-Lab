//! Error type for `tui-lab-pty`.

/// Errors raised by PTY management.
#[derive(Debug)]
pub enum PtyError {
    /// Called before the Phase implementing it lands.
    NotImplemented(&'static str),
    /// The PTY pair or child process could not be created.
    Spawn(String),
    /// I/O with the PTY master failed (read, write, resize, wait).
    Io(String),
    /// A bounded operation (e.g. graceful close) timed out.
    Timeout(String),
    /// Free-form failure with process context attached upstream.
    Message(String),
}

impl std::fmt::Display for PtyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotImplemented(what) => write!(f, "not implemented: {what}"),
            Self::Spawn(msg) => write!(f, "pty spawn failed: {msg}"),
            Self::Io(msg) => write!(f, "pty i/o failed: {msg}"),
            Self::Timeout(msg) => write!(f, "pty operation timed out: {msg}"),
            Self::Message(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for PtyError {}

/// Crate result alias.
pub type Result<T> = std::result::Result<T, PtyError>;
