//! CLI-facing constants (docs/07).

/// Project config file name.
pub const CONFIG_FILE: &str = "tuilab.yaml";
/// Default tests directory.
pub const TESTS_DIR: &str = "tests";
/// Maximum JSON-lines action size (SDK sidecar guard).
pub const MAX_PROTO_LINE_BYTES: usize = 1024 * 1024;
/// CLI exit codes (docs/07 §7.3).
pub const EXIT_OK: i32 = 0;
pub const EXIT_TESTS_FAILED: i32 = 1;
pub const EXIT_CONFIG_ERROR: i32 = 2;
pub const EXIT_PTY_ERROR: i32 = 3;
pub const EXIT_TIMEOUT: i32 = 4;
