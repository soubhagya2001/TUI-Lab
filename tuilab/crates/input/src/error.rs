//! Error type for `tui-lab-input`.

/// Errors raised by input encoding.
#[derive(Debug)]
pub enum InputError {
    /// Called before the Phase implementing it lands.
    NotImplemented(&'static str),
    /// The key name matched nothing (naming helps failure bundles).
    UnknownKey(String),
    /// Free-form failure (e.g. unknown key name) with input context.
    Message(String),
}

impl std::fmt::Display for InputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotImplemented(what) => write!(f, "not implemented: {what}"),
            Self::UnknownKey(name) => write!(f, "unknown key: {name}"),
            Self::Message(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for InputError {}

/// Crate result alias.
pub type Result<T> = std::result::Result<T, InputError>;
