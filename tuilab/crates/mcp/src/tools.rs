//! MCP tool parameter and output shapes (docs/08 §8.1).
//!
//! Every tool returns `Json<Out>` so results are machine-readable; failures
//! are `Err(String)` (MCP tool errors), never silent.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// --- launch ---

/// `tui_launch` input.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LaunchParams {
    /// Executable (must match `security.allow_commands`).
    pub command: String,
    /// CLI arguments.
    #[serde(default)]
    pub args: Vec<String>,
    /// Working directory, jailed under the project root.
    #[serde(default)]
    pub cwd: Option<String>,
    /// Terminal width.
    #[serde(default = "default_width")]
    pub width: u16,
    /// Terminal height.
    #[serde(default = "default_height")]
    pub height: u16,
    /// Extra environment.
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
}

/// `tui_launch` output.
#[derive(Debug, Serialize, JsonSchema)]
pub struct LaunchOut {
    /// New session id.
    pub session_id: String,
    /// Always `"running"` on success.
    pub status: String,
    /// First screen text.
    pub screen: String,
}

fn default_width() -> u16 {
    120
}

fn default_height() -> u16 {
    40
}

// --- press / type ---

/// `tui_press` input.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PressParams {
    /// Session id.
    pub session_id: String,
    /// Key name (`ENTER`, `DOWN`, `CTRL+C`, …). Always follow with
    /// `tui_wait_for_text` — never assert on a stale screen.
    pub key: String,
}

/// `tui_press` output.
#[derive(Debug, Serialize, JsonSchema)]
pub struct PressOut {
    /// Key accepted.
    pub ok: bool,
    /// Whether the screen changed after the key.
    pub screen_changed: bool,
}

/// `tui_type` input.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TypeParams {
    /// Session id.
    pub session_id: String,
    /// Text to type verbatim.
    pub text: String,
    /// Redact from logs (secrets). Always follow with `tui_wait_for_text`.
    #[serde(default)]
    pub sensitive: bool,
}

/// `tui_type` output (never echoes the text).
#[derive(Debug, Serialize, JsonSchema)]
pub struct TypeOut {
    /// Input accepted.
    pub ok: bool,
}

// --- inspect ---

/// `tui_screen` input.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScreenParams {
    /// Session id.
    pub session_id: String,
    /// Request styled cells (accepted; per-cell detail arrives in v2).
    #[serde(default)]
    pub styled: bool,
}

/// Cursor position.
#[derive(Debug, Serialize, JsonSchema)]
pub struct CursorPos {
    /// Zero-based row.
    pub row: usize,
    /// Zero-based column.
    pub col: usize,
}

/// `tui_screen` output.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ScreenOut {
    /// Columns.
    pub width: usize,
    /// Rows.
    pub height: usize,
    /// Cursor position.
    pub cursor: CursorPos,
    /// Plain-text grid.
    pub text: String,
    /// Styled cells, only when `styled: true` was requested.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cells: Option<Vec<ScreenCell>>,
}

/// One styled cell in `tui_screen` output.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ScreenCell {
    /// Zero-based column.
    pub x: usize,
    /// Zero-based row.
    pub y: usize,
    /// Grapheme.
    pub char: String,
    /// Foreground (`black`, `#rrggbb`, `color{n}`, …).
    pub fg: String,
    /// Background, same encoding.
    pub bg: String,
    /// Bold flag.
    pub bold: bool,
    /// Underline flag.
    pub underline: bool,
    /// Reverse-video flag.
    pub reverse: bool,
}

/// `tui_wait_for_text` input.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WaitParams {
    /// Session id.
    pub session_id: String,
    /// Substring to wait for.
    pub text: String,
    /// Regex mode.
    #[serde(default)]
    pub regex: bool,
    /// Timeout in milliseconds.
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

/// `tui_wait_for_text` output.
#[derive(Debug, Serialize, JsonSchema)]
pub struct WaitOut {
    /// Whether the text appeared in time.
    pub found: bool,
    /// Milliseconds waited.
    pub elapsed_ms: u64,
    /// Last screen observed.
    pub screen: String,
}

fn default_timeout_ms() -> u64 {
    3_000
}

/// `tui_assert` input.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AssertParams {
    /// Session id.
    pub session_id: String,
    /// Condition object (`{"type": "text_visible", "text": "..."}`).
    pub assertion: serde_json::Value,
}

/// `tui_assert` output.
#[derive(Debug, Serialize, JsonSchema)]
pub struct AssertOut {
    /// Whether the condition held.
    pub passed: bool,
    /// Human detail (expected vs actual).
    pub detail: String,
}

/// `tui_snapshot` input.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SnapshotParams {
    /// Session id.
    pub session_id: String,
    /// Snapshot name.
    pub name: String,
}

/// `tui_snapshot` output.
#[derive(Debug, Serialize, JsonSchema)]
pub struct SnapshotOut {
    /// True when a new golden was written; false when compared.
    pub saved: bool,
    /// Unified diff on mismatch, else null.
    pub diff: Option<String>,
}

// --- suite + close ---

/// `tui_run_test` input (Mode B: automated execution).
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunTestParams {
    /// Suite file (`tests/login.yaml`; relative paths resolve under root).
    pub test_file: String,
    /// Terminal override.
    #[serde(default)]
    pub terminal: Option<TerminalOverride>,
}

/// Terminal geometry override.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TerminalOverride {
    /// Columns.
    pub width: u16,
    /// Rows.
    pub height: u16,
}

/// One Mode B failure.
#[derive(Debug, Serialize, JsonSchema)]
pub struct RunFailure {
    /// Suite name.
    pub test: String,
    /// Failing step index.
    pub step: usize,
    /// What was expected.
    pub expected: String,
    /// What was observed.
    pub actual: String,
}

/// `tui_run_test` output.
#[derive(Debug, Serialize, JsonSchema)]
pub struct RunTestOut {
    /// `"passed"` or `"failed"`.
    pub status: String,
    /// Passed step count.
    pub passed: usize,
    /// Failed step count.
    pub failed: usize,
    /// Failure details.
    pub failures: Vec<RunFailure>,
}

/// `tui_close` input.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CloseParams {
    /// Session id.
    pub session_id: String,
    /// Optional quit input sent before reaping (e.g. `"q"`). When given,
    /// close polls for natural exit within the grace period and only then
    /// kills — removing the press-quit/close race on slow schedulers.
    #[serde(default)]
    pub quit: Option<String>,
}

/// `tui_close` output.
#[derive(Debug, Serialize, JsonSchema)]
pub struct CloseOut {
    /// Child exited on its own with success.
    pub success: bool,
    /// Numeric exit code as reported.
    pub exit_code: u32,
    /// Termination signal, if reported.
    pub signal: Option<String>,
    /// Kill-path evidence (pump counters), if the grace expired first.
    pub detail: Option<String>,
}
