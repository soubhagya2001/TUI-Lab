//! Assertion condition names (docs/06 §6.1).

/// Text conditions.
pub const COND_TEXT_VISIBLE: &str = "text_visible";
pub const COND_TEXT_NOT_VISIBLE: &str = "text_not_visible";
pub const COND_TEXT_REGEX: &str = "text_regex";
/// Screen conditions.
pub const COND_CURSOR_POSITION: &str = "cursor_position";
pub const COND_SCREEN_CHANGED: &str = "screen_changed";
/// Process conditions.
pub const COND_EXIT_CODE: &str = "exit_code";
pub const COND_NOT_CRASHED: &str = "not_crashed";
