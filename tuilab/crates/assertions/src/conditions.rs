//! Assertion taxonomy over terminal state (docs/06 §6.1).
//!
//! Conditions evaluate against [`ScreenView`] — a plain snapshot of what the
//! engine knows — so every rule is deterministic and PTY-free to test.
//! Performance thresholds (startup/render/latency) are measured-data plumbing
//! for the Phase 3 reporter; enforcement arrives with budgets, not here.

use crate::utils::contains;

/// What the engine knows at assertion time.
pub struct ScreenView {
    /// Plain-text grid dump.
    pub text: String,
    /// Zero-based cursor `(row, col)`.
    pub cursor: (usize, usize),
    /// Whether the screen changed since the last input.
    pub screen_changed: bool,
    /// Process exit code, if the process ended.
    pub exit_code: Option<i32>,
    /// Whether the process died by signal / crash.
    pub crashed: bool,
}

impl ScreenView {
    /// Typical mid-test view: running app, no exit yet.
    pub fn live(text: &str, cursor: (usize, usize), screen_changed: bool) -> Self {
        Self {
            text: text.to_string(),
            cursor,
            screen_changed,
            exit_code: None,
            crashed: false,
        }
    }
}

/// One assertion condition (docs/06 §6.1 taxonomy).
#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    /// `text` is visible somewhere on screen.
    TextVisible(String),
    /// `text` is absent (e.g. `"Unhandled panic"`).
    TextNotVisible(String),
    /// Regex must match screen text.
    TextRegex(String),
    /// Whole screen (trimmed) equals `text`.
    ExactText(String),
    /// Cursor at zero-based `(row, col)`.
    CursorPosition {
        /// Row.
        row: usize,
        /// Column.
        col: usize,
    },
    /// Screen changed (or not) since the last input.
    ScreenChanged(bool),
    /// Process exited with this code.
    ExitCode(i32),
    /// Process did not crash.
    NotCrashed,
}

/// Pass/fail plus a human line for reports and failure bundles.
pub struct Verdict {
    /// Whether the condition held.
    pub passed: bool,
    /// Human-readable detail (expected vs actual).
    pub detail: String,
}

/// Map a JSON assertion object onto a [`Condition`].
///
/// Accepted shapes (`type` discriminates):
/// `text_visible` / `text_not_visible` / `text_regex` / `exact_text` with
/// `text`; `cursor_position` with `row`/`col`; `screen_changed` with optional
/// `changed` (default true); `exit_code` with `code`; `not_crashed`.
/// Shared by the MCP server and the `tuilab proto` sidecar mode.
pub fn condition_from_json(value: &serde_json::Value) -> Result<Condition, String> {
    let obj = value
        .as_object()
        .ok_or_else(|| "assertion must be an object".to_string())?;
    let kind = obj
        .get("type")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "assertion needs a string \"type\"".to_string())?;
    let text_field = |name: &str| {
        obj.get(name)
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| format!("{kind} needs a string {name:?}"))
    };
    match kind {
        "text_visible" => Ok(Condition::TextVisible(text_field("text")?)),
        "text_not_visible" => Ok(Condition::TextNotVisible(text_field("text")?)),
        "text_regex" => Ok(Condition::TextRegex(text_field("text")?)),
        "exact_text" => Ok(Condition::ExactText(text_field("text")?)),
        "cursor_position" => {
            let row = obj
                .get("row")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| "cursor_position needs numeric row".to_string())?
                as usize;
            let col = obj
                .get("col")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| "cursor_position needs numeric col".to_string())?
                as usize;
            Ok(Condition::CursorPosition { row, col })
        }
        "screen_changed" => {
            let expected = obj
                .get("changed")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true);
            Ok(Condition::ScreenChanged(expected))
        }
        "exit_code" => {
            let code = obj
                .get("code")
                .and_then(serde_json::Value::as_i64)
                .ok_or_else(|| "exit_code needs numeric code".to_string())?
                as i32;
            Ok(Condition::ExitCode(code))
        }
        "not_crashed" => Ok(Condition::NotCrashed),
        other => Err(format!("unknown assertion type: {other}")),
    }
}
/// Evaluate one condition against a view. Never panics — a bad regex is a
/// failed verdict, since failures must feed reports, not crash runners.
pub fn evaluate(condition: &Condition, view: &ScreenView) -> Verdict {
    let fail = |detail: String| Verdict {
        passed: false,
        detail,
    };
    let pass = |detail: String| Verdict {
        passed: true,
        detail,
    };
    match condition {
        Condition::TextVisible(needle) => {
            if contains(&view.text, needle) {
                pass(format!("text visible: {needle:?}"))
            } else {
                fail(format!("expected visible text {needle:?}"))
            }
        }
        Condition::TextNotVisible(needle) => {
            if contains(&view.text, needle) {
                fail(format!("unexpected visible text {needle:?}"))
            } else {
                pass(format!("text absent: {needle:?}"))
            }
        }
        Condition::TextRegex(pattern) => match regex::Regex::new(pattern) {
            Ok(matched) if matched.is_match(&view.text) => {
                pass(format!("regex matched: {pattern:?}"))
            }
            Ok(_) => fail(format!("regex did not match: {pattern:?}")),
            Err(e) => fail(format!("invalid regex {pattern:?}: {e}")),
        },
        Condition::ExactText(expected) => {
            if view.text.trim() == expected.trim() {
                pass("exact text matched".to_string())
            } else {
                fail(format!("exact text mismatch: expected {expected:?}"))
            }
        }
        Condition::CursorPosition { row, col } => {
            if view.cursor == (*row, *col) {
                pass(format!("cursor at {row}:{col}"))
            } else {
                fail(format!("cursor at {:?}, expected {row}:{col}", view.cursor))
            }
        }
        Condition::ScreenChanged(expected) => {
            if view.screen_changed == *expected {
                pass(format!("screen_changed == {expected}"))
            } else {
                fail(format!(
                    "screen_changed == {}, expected {expected}",
                    view.screen_changed
                ))
            }
        }
        Condition::ExitCode(expected) => match view.exit_code {
            Some(code) if code == *expected => pass(format!("exit code {code}")),
            Some(code) => fail(format!("exit code {code}, expected {expected}")),
            None => fail("process still running; no exit code yet".to_string()),
        },
        Condition::NotCrashed => {
            if view.crashed {
                fail("process crashed".to_string())
            } else {
                pass("process healthy".to_string())
            }
        }
    }
}
