//! Error type for `tui-lab-snapshots`.

/// Errors raised by snapshot capture, storage, and comparison.
#[derive(Debug)]
pub enum SnapshotError {
    /// Called before the Phase implementing it lands.
    NotImplemented(&'static str),
    /// Free-form failure with snapshot-name context attached upstream.
    Message(String),
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotImplemented(what) => write!(f, "not implemented: {what}"),
            Self::Message(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for SnapshotError {}

/// Crate result alias.
pub type Result<T> = std::result::Result<T, SnapshotError>;
