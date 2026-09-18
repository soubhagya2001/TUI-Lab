//! PTY defaults (docs/03 §3.4, docs/12 §12.3).

/// Default `$TERM` advertised to child processes.
pub const DEFAULT_TERM: &str = "xterm-256color";
/// Default terminal width in columns.
pub const DEFAULT_WIDTH: u16 = 120;
/// Default terminal height in rows.
pub const DEFAULT_HEIGHT: u16 = 40;
/// Minimum supported Windows 10 release for ConPTY.
pub const MIN_WINDOWS_BUILD: u32 = 17_109;
/// Default grace period before SIGKILL / TerminateProcess in `close`.
///
/// Generous on purpose: the grace only elapses on the kill path, so slow
/// schedulers (loaded CI) get room while cooperative closes return at once.
pub const KILL_GRACE_DEFAULT_MS: u64 = 10_000;
