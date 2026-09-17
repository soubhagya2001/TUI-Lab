//! Error type for `tui-lab-reporter`.

/// Errors raised while writing reports.
#[derive(Debug)]
pub enum ReporterError {
    /// Called before the Phase implementing it lands.
    NotImplemented(&'static str),
    /// Free-form failure with report-path context attached upstream.
    Message(String),
}

impl std::fmt::Display for ReporterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotImplemented(what) => write!(f, "not implemented: {what}"),
            Self::Message(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for ReporterError {}

/// Crate result alias.
pub type Result<T> = std::result::Result<T, ReporterError>;
