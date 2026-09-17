//! Protocol versions and action names (docs/04).

/// Canonical schema id used in every YAML test file.
pub const SCHEMA_ID: &str = "tui-lab/v1";
/// Protocol version negotiated on session creation.
pub const PROTOCOL_VERSION: &str = "1.0";
/// Action names (v1 closed set).
pub const ACTION_LAUNCH: &str = "launch";
pub const ACTION_PRESS: &str = "press";
pub const ACTION_TYPE: &str = "type";
pub const ACTION_WAIT_FOR_TEXT: &str = "wait_for_text";
pub const ACTION_SCREEN: &str = "screen";
pub const ACTION_ASSERT: &str = "assert";
pub const ACTION_SNAPSHOT: &str = "snapshot";
pub const ACTION_RESIZE: &str = "resize";
pub const ACTION_CLOSE: &str = "close";
