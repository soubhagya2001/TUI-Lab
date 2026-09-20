//! Error type for `tui-lab-protocol`.

/// Errors raised while parsing or validating protocol payloads.
#[derive(Debug)]
pub enum ProtocolError {
    /// Malformed JSON/YAML or shape mismatch.
    Schema(String),
    /// Unsupported `schema:` id (breaking change → new version).
    Version(String),
    /// Free-form failure with payload context attached upstream.
    Message(String),
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Schema(msg) => write!(f, "protocol schema error: {msg}"),
            Self::Version(id) => write!(f, "unsupported schema version: {id}"),
            Self::Message(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for ProtocolError {}

/// Crate result alias.
pub type Result<T> = std::result::Result<T, ProtocolError>;
