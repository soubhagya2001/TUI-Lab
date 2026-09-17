//! Named-key table (docs/03 §3.3, docs/04 §4.2).
//!
//! Full byte encodings arrive in Phase 1; names are frozen here so YAML/MCP
//! authors can rely on them.

/// Canonical key names accepted by `press` (case-insensitive).
pub const KEY_ENTER: &str = "ENTER";
pub const KEY_ESCAPE: &str = "ESC";
pub const KEY_TAB: &str = "TAB";
pub const KEY_BACKTAB: &str = "BACKTAB";
pub const KEY_UP: &str = "UP";
pub const KEY_DOWN: &str = "DOWN";
pub const KEY_LEFT: &str = "LEFT";
pub const KEY_RIGHT: &str = "RIGHT";
pub const KEY_HOME: &str = "HOME";
pub const KEY_END: &str = "END";
pub const KEY_PAGE_UP: &str = "PGUP";
pub const KEY_PAGE_DOWN: &str = "PGDN";
pub const KEY_INSERT: &str = "INSERT";
pub const KEY_DELETE: &str = "DELETE";
