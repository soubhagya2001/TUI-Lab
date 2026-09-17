//! Session and lifecycle defaults (docs/02 §2.4, docs/08 §8.4).

/// Default per-wait timeout.
pub const DEFAULT_TIMEOUT_MS: u64 = 5_000;
/// Screen poll interval for `wait_for_text`.
pub const DEFAULT_POLL_MS: u64 = 50;
/// Maximum concurrent PTY sessions (MCP + CLI runners share the budget).
pub const MAX_SESSIONS: usize = 8;
/// Idle sessions are reaped after this long.
pub const IDLE_TIMEOUT_SECS: u64 = 60;
