//! `TestContext`: per-run session state (docs/02 §2.4).
//!
//! Every error leaving the runner carries step/session context from here,
//! feeding the failure bundle (docs/11 §11.3).

use std::time::Instant;

/// Live state for one suite execution (single session in Phase 3).
pub struct TestContext {
    /// Session id (`sess_001`, …).
    pub session_id: String,
    /// Suite name from YAML.
    pub suite: String,
    /// Run start time.
    pub started_at: Instant,
    /// Index of the step currently executing.
    pub step_index: usize,
    /// Human-readable input history (`press ENTER`, `type[len=5]`, …).
    pub input_history: Vec<String>,
}

impl TestContext {
    /// Start a context for `suite` with the given session id.
    pub fn new(session_id: &str, suite: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            suite: suite.to_string(),
            started_at: Instant::now(),
            step_index: 0,
            input_history: Vec::new(),
        }
    }

    /// Milliseconds since the run started.
    pub fn elapsed_ms(&self) -> u64 {
        self.started_at.elapsed().as_millis() as u64
    }
}
