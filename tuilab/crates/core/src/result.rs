//! Serializable run outcome: what the reporter formats (docs/11).
//!
//! Pure data — the reporter stays a formatter (thin-adapter rule).

use serde::{Deserialize, Serialize};

/// Whole-suite outcome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuiteResult {
    /// Schema id (`tui-lab/v1`).
    pub schema: String,
    /// Suite name.
    pub suite: String,
    /// All steps passed and exit assertions held.
    pub passed: bool,
    /// Process exit success, if the process ended.
    pub exit_success: Option<bool>,
    /// Termination signal name, if reported (e.g. `SIGKILL`).
    pub exit_signal: Option<String>,
    /// Total run time in milliseconds.
    pub duration_ms: u64,
    /// Per-step outcomes, in execution order.
    pub steps: Vec<StepResult>,
    /// First failure details (failure bundle core).
    pub failure: Option<FailureInfo>,
    /// Terminal the suite ran under.
    pub terminal: TerminalInfo,
}

/// One executed step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepResult {
    /// Zero-based index across setup+steps+cleanup.
    pub index: usize,
    /// Human step description (`press DOWN`, `wait_for_text "x"`, …).
    pub kind: String,
    /// Whether the step passed.
    pub passed: bool,
    /// Detail line (verdict text, diff head, timings).
    pub detail: String,
    /// Step duration in milliseconds.
    pub duration_ms: u64,
}

/// First-failure evidence (docs/11 §11.3, Phase 3 subset).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FailureInfo {
    /// Step index that failed.
    pub step_index: usize,
    /// Human step description.
    pub step: String,
    /// What was expected.
    pub expected: String,
    /// What was observed.
    pub actual: String,
    /// Last full screen.
    pub last_screen: String,
    /// Input history up to the failure.
    pub input_history: Vec<String>,
}

/// Terminal geometry + type for the run record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalInfo {
    /// Columns.
    pub width: u16,
    /// Rows.
    pub height: u16,
    /// `$TERM` value advertised.
    pub term: String,
}
