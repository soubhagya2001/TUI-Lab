//! Error type for `tui-lab-runtime`.

/// Errors raised by the runtime supervisor.
#[derive(Debug)]
pub enum RuntimeError {
    /// Called before the Phase implementing it lands.
    NotImplemented(&'static str),
    /// Free-form failure with timeout/attempt context attached upstream.
    Message(String),
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotImplemented(what) => write!(f, "not implemented: {what}"),
            Self::Message(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for RuntimeError {}

/// Crate result alias.
pub type Result<T> = std::result::Result<T, RuntimeError>;
