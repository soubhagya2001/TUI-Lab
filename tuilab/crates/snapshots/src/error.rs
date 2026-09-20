//! Error type for `tui-lab-snapshots`.

/// Errors raised by snapshot capture, storage, and comparison.
#[derive(Debug)]
pub enum SnapshotError {
    /// Filesystem failure (save/load golden).
    Io(String),
    /// Free-form failure with snapshot-name context attached upstream.
    Message(String),
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(msg) => write!(f, "snapshot i/o failed: {msg}"),
            Self::Message(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for SnapshotError {}

/// Crate result alias.
pub type Result<T> = std::result::Result<T, SnapshotError>;
