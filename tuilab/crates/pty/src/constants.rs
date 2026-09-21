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

/// Env keys never passed to children (S4): exact matches, case-insensitive
/// (`Path` on Windows). `PATH` passthrough would let a suite redirect the
/// child to attacker-controlled binaries.
pub const BLOCKED_ENV_EXACT: &[&str] = &["PATH"];
/// Env key prefixes never passed to children (S4): loader hijack (`LD_*`,
/// `DYLD_*`) and toolchain poisoning (`CARGO_*`). Matched case-insensitively
/// so `Ld_Preload` cannot slip through either.
pub const BLOCKED_ENV_PREFIXES: &[&str] = &["LD_", "DYLD_", "CARGO_"];
