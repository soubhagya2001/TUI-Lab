//! CLI-facing constants (docs/07).

/// Project config file name.
pub const CONFIG_FILE: &str = "tuilab.yaml";
/// Default tests directory.
pub const TESTS_DIR: &str = "tests";
/// Maximum JSON-lines action size (SDK sidecar guard).
pub const MAX_PROTO_LINE_BYTES: usize = 1024 * 1024;
/// Recorder: main-loop tick (pump + render + input drain).
pub const REC_TICK_MS: u64 = 50;
/// Recorder: max stdin bytes consumed per tick (paces beats for synthesis).
pub const REC_CHUNK_BYTES: usize = 8;
/// Recorder: consecutive identical screens that count as stable.
pub const REC_STABLE_POLLS: usize = 3;
/// Recorder: give up waiting for stability after this long.
pub const REC_STABILIZE_TIMEOUT_MS: u64 = 5_000;
/// Recorder: idle wait for the next input beat before pumping on.
pub const REC_BEAT_WAIT_MS: u64 = 100;
/// Recorder: byte that ends the session without forwarding (Ctrl+\).
pub const REC_FINISH_BYTE: u8 = 0x1c;
/// CLI exit codes (docs/07 §7.3).
pub const EXIT_OK: i32 = 0;
pub const EXIT_TESTS_FAILED: i32 = 1;
pub const EXIT_CONFIG_ERROR: i32 = 2;
pub const EXIT_PTY_ERROR: i32 = 3;
pub const EXIT_TIMEOUT: i32 = 4;
