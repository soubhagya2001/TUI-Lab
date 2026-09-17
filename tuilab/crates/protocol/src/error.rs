//! Error type for `tui-lab-protocol`.

/// Errors raised while parsing or validating protocol payloads.
#[derive(Debug)]
pub enum ProtocolError {
    /// Called before the Phase implementing it lands.
    NotImplemented(&'static str),
    /// Free-form failure with payload context attached upstream.
    Message(String),
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotImplemented(what) => write!(f, "not implemented: {what}"),
            Self::Message(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for ProtocolError {}

/// Crate result alias.
pub type Result<T> = std::result::Result<T, ProtocolError>;
