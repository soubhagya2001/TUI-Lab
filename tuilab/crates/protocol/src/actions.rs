//! v1 action envelope: the 9 actions every frontend speaks (docs/04 §4.2).
//!
//! Wire shape is JSON (`{"action": "press", ...}`); all frontends (CLI, MCP,
//! SDKs) build these types rather than hand-rolling payloads.

use serde::{Deserialize, Serialize};

use crate::constants::{
    ACTION_ASSERT, ACTION_CLOSE, ACTION_LAUNCH, ACTION_PRESS, ACTION_RESIZE, ACTION_SCREEN,
    ACTION_SNAPSHOT, ACTION_TYPE, ACTION_WAIT_FOR_TEXT,
};
use crate::error::{ProtocolError, Result};

/// Default `wait_for_text` timeout (docs/06 §6.4).
pub const DEFAULT_WAIT_TIMEOUT_MS: u64 = 3_000;
/// Default screen poll interval.
pub const DEFAULT_POLL_MS: u64 = 50;

/// Terminal geometry for `launch`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalSize {
    /// Columns.
    pub width: u16,
    /// Rows.
    pub height: u16,
}

/// The 9 v1 actions, tagged by `action`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    /// Start a child under a fresh PTY; returns `session_id` + first screen.
    Launch {
        /// Executable.
        command: String,
        /// CLI arguments.
        #[serde(default)]
        args: Vec<String>,
        /// Working directory. Must resolve under the project root (MCP jail).
        #[serde(default)]
        cwd: Option<String>,
        /// Terminal geometry.
        #[serde(default)]
        terminal: Option<TerminalSize>,
        /// Extra environment (`TERM` defaults downstream).
        #[serde(default)]
        env: std::collections::HashMap<String, String>,
        /// Launch timeout.
        #[serde(default = "default_launch_timeout")]
        timeout_ms: u64,
    },
    /// Send a named key (`ENTER`, `DOWN`, `CTRL+C`, …).
    Press {
        /// Session id.
        session_id: String,
        /// Key name (case-insensitive; see `tui-lab-input`).
        key: String,
    },
    /// Type text verbatim (no Enter appended).
    Type {
        /// Session id.
        session_id: String,
        /// Text to type.
        text: String,
        /// When true, redact from logs (secrets).
        #[serde(default)]
        sensitive: bool,
    },
    /// Poll the screen until text appears (preferred over sleep).
    WaitForText {
        /// Session id.
        session_id: String,
        /// Substring to wait for.
        text: String,
        /// Treat `text` as a regex.
        #[serde(default)]
        regex: bool,
        /// Give up after this long.
        #[serde(default = "default_wait_timeout")]
        timeout_ms: u64,
        /// Poll interval.
        #[serde(default = "default_poll")]
        poll_ms: u64,
    },
    /// Return the current screen (text + optional cells).
    Screen {
        /// Session id.
        session_id: String,
        /// Include per-cell style data.
        #[serde(default)]
        styled: bool,
    },
    /// Evaluate one assertion condition (see `tui-lab-assertions`).
    Assert {
        /// Session id.
        session_id: String,
        /// Condition payload (tagged by `type`).
        condition: serde_json::Value,
    },
    /// Capture a named snapshot for golden comparison.
    Snapshot {
        /// Session id.
        session_id: String,
        /// Snapshot name.
        name: String,
    },
    /// Resize the PTY + grid. Follow with `wait_for_text` (async redraw).
    Resize {
        /// Session id.
        session_id: String,
        /// Columns.
        width: u16,
        /// Rows.
        height: u16,
    },
    /// Quit the app (optional input first) and reap the child.
    Close {
        /// Session id.
        session_id: String,
        /// Quit input to send before waiting (e.g. `q`).
        #[serde(default)]
        signal: Option<String>,
        /// Give up waiting after this long, then kill.
        #[serde(default = "default_wait_timeout")]
        timeout_ms: u64,
    },
}

fn default_launch_timeout() -> u64 {
    10_000
}

fn default_wait_timeout() -> u64 {
    DEFAULT_WAIT_TIMEOUT_MS
}

fn default_poll() -> u64 {
    DEFAULT_POLL_MS
}

/// Allowed fields per action (strict v1 mode: anything else is rejected).
///
/// Internally-tagged enums cannot use `deny_unknown_fields`, so
/// [`Action::from_json`] validates against this table first. The same table
/// serves MCP tool validation in Phase 4 — reuse it, don't copy it.
const ACTION_FIELDS: &[(&str, &[&str])] = &[
    (
        "launch",
        &[
            "action",
            "command",
            "args",
            "cwd",
            "terminal",
            "env",
            "timeout_ms",
        ],
    ),
    ("press", &["action", "session_id", "key"]),
    ("type", &["action", "session_id", "text", "sensitive"]),
    (
        "wait_for_text",
        &[
            "action",
            "session_id",
            "text",
            "regex",
            "timeout_ms",
            "poll_ms",
        ],
    ),
    ("screen", &["action", "session_id", "styled"]),
    ("assert", &["action", "session_id", "condition"]),
    ("snapshot", &["action", "session_id", "name"]),
    ("resize", &["action", "session_id", "width", "height"]),
    ("close", &["action", "session_id", "signal", "timeout_ms"]),
];

impl Action {
    /// The action discriminant, matching `docs/04` names.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Launch { .. } => ACTION_LAUNCH,
            Self::Press { .. } => ACTION_PRESS,
            Self::Type { .. } => ACTION_TYPE,
            Self::WaitForText { .. } => ACTION_WAIT_FOR_TEXT,
            Self::Screen { .. } => ACTION_SCREEN,
            Self::Assert { .. } => ACTION_ASSERT,
            Self::Snapshot { .. } => ACTION_SNAPSHOT,
            Self::Resize { .. } => ACTION_RESIZE,
            Self::Close { .. } => ACTION_CLOSE,
        }
    }

    /// Deserialize one JSON action line, rejecting unknown actions and,
    /// in strict v1 mode, unknown fields.
    pub fn from_json(line: &str) -> Result<Self> {
        let value: serde_json::Value =
            serde_json::from_str(line).map_err(|e| ProtocolError::Schema(e.to_string()))?;
        let obj = value
            .as_object()
            .ok_or_else(|| ProtocolError::Schema("action must be a JSON object".to_string()))?;
        let name = obj
            .get("action")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ProtocolError::Schema("missing action field".to_string()))?;
        let (_, allowed) = ACTION_FIELDS
            .iter()
            .find(|(action, _)| *action == name)
            .ok_or_else(|| ProtocolError::Schema(format!("unknown action: {name}")))?;
        for key in obj.keys() {
            if !allowed.contains(&key.as_str()) {
                return Err(ProtocolError::Schema(format!(
                    "unknown field {key:?} for action {name:?}"
                )));
            }
        }
        serde_json::from_value(value).map_err(|e| ProtocolError::Schema(e.to_string()))
    }

    /// Serialize to canonical JSON.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).map_err(|e| ProtocolError::Schema(e.to_string()))
    }
}
