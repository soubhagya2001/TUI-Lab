//! Supervisor defaults: retries, backoff, parallelism (docs/02 §2.2).

/// How often the supervisor polls the screen buffer.
pub const POLL_MS: u64 = 50;
/// Grace period before SIGKILL / TerminateProcess after SIGTERM.
pub const KILL_GRACE_MS: u64 = 2_000;
/// Default parallel test slots (overridable via `tuilab.yaml`).
pub const DEFAULT_PARALLEL: usize = 4;
