//! MCP server constants (docs/08).

/// Rejection code when a command misses the allowlist.
pub const FORBIDDEN_COMMAND: &str = "FORBIDDEN_COMMAND";
/// Default session idle timeout before reaping.
pub const SESSION_IDLE_SECS: u64 = 60;
/// Default cap on concurrent sessions.
pub const MAX_SESSIONS: usize = 8;
