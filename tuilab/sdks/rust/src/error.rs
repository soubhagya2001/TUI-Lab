//! Engine or transport failure, with step/session context attached.

/// Engine or transport failure. `detail` carries the reply or context.
#[derive(Debug)]
pub struct TuiLabError {
    /// Human message.
    pub message: String,
    /// Raw reply or context, when available.
    pub detail: Option<serde_json::Value>,
}

impl TuiLabError {
    /// Build from a message with no detail.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            detail: None,
        }
    }

    /// Build from a message plus reply context.
    pub fn with_detail(message: impl Into<String>, detail: serde_json::Value) -> Self {
        Self {
            message: message.into(),
            detail: Some(detail),
        }
    }
}

impl std::fmt::Display for TuiLabError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for TuiLabError {}
