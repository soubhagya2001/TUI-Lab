//! MCP handler: the 9 `tui_*` tools over shared core state (docs/08).
//!
//! Modes: A (interactive launch → inspect → act loop) and B (`tui_run_test`
//! full suites). Every input tool's description ends with the sync rule:
//! follow it with `tui_wait_for_text`, never assert on a stale screen.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_handler, tool_router, ServerHandler};
use tui_lab_assertions::{condition_from_json, evaluate};
use tui_lab_core::{run_file, NewSession, RunOptions, SessionRegistry};
use tui_lab_input::{encode_key, encode_text};
use tui_lab_protocol::TestFile;
use tui_lab_pty::SpawnOptions;
use tui_lab_runtime::wait_for_text;

use crate::constants::{MAX_SESSIONS, SESSION_IDLE_SECS};
use crate::security::{jail, Allowlist};
use crate::tools::{
    AssertOut, AssertParams, CloseOut, CloseParams, CursorPos, LaunchOut, LaunchParams, PressOut,
    PressParams, RunFailure, RunTestOut, RunTestParams, ScreenOut, ScreenParams, SnapshotOut,
    SnapshotParams, TypeOut, TypeParams, WaitOut, WaitParams,
};

/// Reap sessions idle longer than this on every launch.
const IDLE_REAP_SECS: u64 = SESSION_IDLE_SECS;

/// Short pump budget after single inputs.
const INPUT_SETTLE_MS: u64 = 300;

/// Shared server state behind one async mutex.
pub struct HandlerState {
    /// Live PTY sessions (core-owned).
    pub registry: SessionRegistry,
    /// Project root (cwd jail + suite resolution).
    pub root: PathBuf,
    /// Launch allowlist.
    pub allow: Allowlist,
}

/// `tuilab-mcp` request handler.
#[derive(Clone)]
pub struct TuiLabHandler {
    state: Arc<tokio::sync::Mutex<HandlerState>>,
    tool_router: ToolRouter<Self>,
}

impl TuiLabHandler {
    /// Build a handler rooted at `root` with the given allowlist.
    pub fn new(root: PathBuf, allow: Allowlist) -> Self {
        let state = Arc::new(tokio::sync::Mutex::new(HandlerState {
            registry: SessionRegistry::with_cap(MAX_SESSIONS),
            root,
            allow,
        }));
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }

    /// Expose the router for `list_all` introspection (tests, diagnostics).
    pub fn router(&self) -> &ToolRouter<Self> {
        &self.tool_router
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for TuiLabHandler {}

/// Map registry/core errors to MCP tool errors.
fn tool_error(context: &str, message: impl std::fmt::Display) -> String {
    format!("{context}: {message}")
}

#[tool_router]
impl TuiLabHandler {
    /// Launch an application under a fresh PTY. The command must match
    /// security.allow_commands and cwd must stay under the project root.
    /// Then call tui_wait_for_text — never assert on a stale screen.
    #[tool(
        name = "tui_launch",
        description = "Launch a terminal application under a PTY. Returns session_id plus the first screen. Then call tui_wait_for_text."
    )]
    pub async fn tui_launch(
        &self,
        params: Parameters<LaunchParams>,
    ) -> Result<Json<LaunchOut>, String> {
        let params = params.0;
        let mut state = self.state.lock().await;
        state
            .allow
            .check(&params.command, &params.args)
            .map_err(|e| tool_error("launch", e))?;
        let cwd = jail(&state.root, params.cwd.as_deref()).map_err(|e| tool_error("launch", e))?;
        state
            .registry
            .reap_idle(Duration::from_secs(IDLE_REAP_SECS));
        let id = state
            .registry
            .spawn(NewSession::new(SpawnOptions {
                command: params.command,
                args: params.args,
                cwd: Some(cwd),
                env: params.env.into_iter().collect(),
                cols: params.width,
                rows: params.height,
            }))
            .map_err(|e| tool_error("launch", e))?;
        let session = state
            .registry
            .get_mut(&id)
            .map_err(|e| tool_error("launch", e))?;
        let screen = SessionRegistry::pump_once(session, Duration::from_millis(500));
        Ok(Json(LaunchOut {
            session_id: id,
            status: "running".to_string(),
            screen,
        }))
    }

    /// Send a named key (ENTER, DOWN, CTRL+C, …). Then call tui_wait_for_text.
    #[tool(
        name = "tui_press",
        description = "Press a named key in a session. Then call tui_wait_for_text — never assert on a stale screen."
    )]
    pub async fn tui_press(
        &self,
        params: Parameters<PressParams>,
    ) -> Result<Json<PressOut>, String> {
        let params = params.0;
        let mut state = self.state.lock().await;
        let session = state
            .registry
            .get_mut(&params.session_id)
            .map_err(|e| tool_error("press", e))?;
        let bytes = encode_key(&params.key)
            .map_err(|e| tool_error("press", format!("{:?}: {e}", params.key)))?;
        let before = session.emu.text();
        session
            .pty
            .write_all(&bytes)
            .map_err(|e| tool_error("press", e))?;
        session.input_history.push(format!("press {}", params.key));
        let text = SessionRegistry::pump_once(session, Duration::from_millis(INPUT_SETTLE_MS));
        Ok(Json(PressOut {
            ok: true,
            screen_changed: text != before,
        }))
    }

    /// Type text verbatim. Set sensitive for secrets (redacted from logs).
    /// Then call tui_wait_for_text.
    #[tool(
        name = "tui_type",
        description = "Type text verbatim in a session. Set sensitive=true for secrets (only the length is logged). Then call tui_wait_for_text."
    )]
    pub async fn tui_type(&self, params: Parameters<TypeParams>) -> Result<Json<TypeOut>, String> {
        let params = params.0;
        let mut state = self.state.lock().await;
        let session = state
            .registry
            .get_mut(&params.session_id)
            .map_err(|e| tool_error("type", e))?;
        session
            .pty
            .write_all(&encode_text(&params.text))
            .map_err(|e| tool_error("type", e))?;
        if params.sensitive {
            tracing::info!(len = params.text.len(), "typed sensitive input");
            session
                .input_history
                .push("type[len=n, redacted]".to_string());
        } else {
            session
                .input_history
                .push(format!("type {:?}", params.text));
        }
        Ok(Json(TypeOut { ok: true }))
    }

    /// Read the current screen grid as text.
    #[tool(
        name = "tui_screen",
        description = "Read the current terminal screen (text, cursor, dimensions)."
    )]
    pub async fn tui_screen(
        &self,
        params: Parameters<ScreenParams>,
    ) -> Result<Json<ScreenOut>, String> {
        let params = params.0;
        let mut state = self.state.lock().await;
        let session = state
            .registry
            .get_mut(&params.session_id)
            .map_err(|e| tool_error("screen", e))?;
        let text = SessionRegistry::pump_once(session, Duration::from_millis(100));
        let (width, height) = session.emu.dims();
        let (row, col) = session.emu.cursor();
        let _ = params.styled; // Per-cell detail arrives in v2; text rules v1.
        Ok(Json(ScreenOut {
            width,
            height,
            cursor: CursorPos { row, col },
            text,
        }))
    }

    /// Poll until text appears (or the timeout elapses). Prefer this over any
    /// fixed sleep — TUI redraw is asynchronous.
    #[tool(
        name = "tui_wait_for_text",
        description = "Wait until text appears on screen (polling with timeout). Use after every input instead of sleeping."
    )]
    pub async fn tui_wait_for_text(
        &self,
        params: Parameters<WaitParams>,
    ) -> Result<Json<WaitOut>, String> {
        let params = params.0;
        let mut state = self.state.lock().await;
        let session = state
            .registry
            .get_mut(&params.session_id)
            .map_err(|e| tool_error("wait", e))?;
        let outcome = wait_for_text(
            || SessionRegistry::pump_once(session, Duration::from_millis(50)),
            &params.text,
            params.regex,
            Duration::from_millis(params.timeout_ms),
            Duration::from_millis(50),
        )
        .await;
        Ok(Json(WaitOut {
            found: outcome.found,
            elapsed_ms: outcome.elapsed.as_millis() as u64,
            screen: outcome.last_screen,
        }))
    }

    /// Evaluate one assertion against the live screen. Prefer waiting first
    /// with tui_wait_for_text so the verdict reflects the latest state.
    #[tool(
        name = "tui_assert",
        description = "Assert a condition (text_visible, text_not_visible, text_regex, cursor_position, screen_changed, exit_code, not_crashed) against the live screen."
    )]
    pub async fn tui_assert(
        &self,
        params: Parameters<AssertParams>,
    ) -> Result<Json<AssertOut>, String> {
        let params = params.0;
        let mut state = self.state.lock().await;
        let session = state
            .registry
            .get_mut(&params.session_id)
            .map_err(|e| tool_error("assert", e))?;
        let text = SessionRegistry::pump_once(session, Duration::from_millis(100));
        let condition =
            condition_from_json(&params.assertion).map_err(|e| tool_error("assert", e))?;
        let view = live_view(session, &text);
        let verdict = evaluate(&condition, &view);
        Ok(Json(AssertOut {
            passed: verdict.passed,
            detail: verdict.detail,
        }))
    }

    /// Capture a named snapshot: writes a golden on first use, compares after.
    #[tool(
        name = "tui_snapshot",
        description = "Snapshot the screen. First call writes the golden (saved=true); later calls compare and return a diff on mismatch."
    )]
    pub async fn tui_snapshot(
        &self,
        params: Parameters<SnapshotParams>,
    ) -> Result<Json<SnapshotOut>, String> {
        let params = params.0;
        let mut state = self.state.lock().await;
        let session = state
            .registry
            .get_mut(&params.session_id)
            .map_err(|e| tool_error("snapshot", e))?;
        let text = SessionRegistry::pump_once(session, Duration::from_millis(100));
        let (width, height) = session.emu.dims();
        let dir = state.root.join("tests/snapshots/mcp");
        let golden =
            tui_lab_snapshots::text_golden_path(&dir, &params.name, width as u16, height as u16);
        if !golden.exists() {
            tui_lab_snapshots::save_text(&dir, &params.name, width as u16, height as u16, &text)
                .map_err(|e| tool_error("snapshot", e))?;
            return Ok(Json(SnapshotOut {
                saved: true,
                diff: None,
            }));
        }
        let expected =
            tui_lab_snapshots::load_text(&golden).map_err(|e| tool_error("snapshot", e))?;
        let outcome = tui_lab_snapshots::compare_text(&expected, &text, &[]);
        Ok(Json(SnapshotOut {
            saved: false,
            diff: (!outcome.equal).then_some(outcome.diff),
        }))
    }

    /// Run a full YAML suite (Mode B automated execution). Prefer this for
    /// regression; use the interactive tools for investigation.
    #[tool(
        name = "tui_run_test",
        description = "Run a YAML test suite end to end and return pass/fail plus failures. Prefer for regression runs."
    )]
    pub async fn tui_run_test(
        &self,
        params: Parameters<RunTestParams>,
    ) -> Result<Json<RunTestOut>, String> {
        let params = params.0;
        let state = self.state.lock().await;
        let suite_path = if Path::new(&params.test_file).is_absolute() {
            PathBuf::from(&params.test_file)
        } else {
            state.root.join(&params.test_file)
        };
        let text = std::fs::read_to_string(&suite_path)
            .map_err(|e| tool_error("run_test", format!("{}: {e}", suite_path.display())))?;
        let mut file = TestFile::from_yaml(&text).map_err(|e| tool_error("run_test", e))?;
        if let Some(terminal) = params.terminal {
            file.terminal.width = terminal.width;
            file.terminal.height = terminal.height;
        }
        drop(state);
        let snapshot_dir = self.snapshot_dir().await;
        let opts = RunOptions {
            snapshot_dir,
            ..RunOptions::default()
        };
        let result = run_file(&file, &opts)
            .await
            .map_err(|e| tool_error("run_test", e))?;
        let failed_steps: Vec<_> = result.steps.iter().filter(|step| !step.passed).collect();
        let failures = result
            .failure
            .as_ref()
            .map(|failure| {
                vec![RunFailure {
                    test: result.suite.clone(),
                    step: failure.step_index,
                    expected: failure.expected.clone(),
                    actual: failure.actual.clone(),
                }]
            })
            .unwrap_or_default();
        Ok(Json(RunTestOut {
            status: if result.passed {
                "passed".to_string()
            } else {
                "failed".to_string()
            },
            passed: result.steps.len() - failed_steps.len(),
            failed: failed_steps.len(),
            failures,
        }))
    }

    /// Close a session, reaping the child. Always close what you launch.
    /// Pass `quit` (e.g. "q") so close polls for natural exit first and only
    /// kills on timeout — this beats press-quit-then-close races on loaded CI.
    #[tool(
        name = "tui_close",
        description = "Close a session and reap its process, optionally sending quit input first and waiting for natural exit. Always close sessions you launch."
    )]
    pub async fn tui_close(
        &self,
        params: Parameters<CloseParams>,
    ) -> Result<Json<CloseOut>, String> {
        let params = params.0;
        let mut state = self.state.lock().await;
        let closed = state
            .registry
            .remove(
                &params.session_id,
                params.quit.as_deref().map(str::as_bytes),
            )
            .map_err(|e| tool_error("close", e))?;
        Ok(Json(CloseOut {
            success: closed.exited_cleanly,
            signal: closed.signal,
        }))
    }
}

impl TuiLabHandler {
    /// Snapshot dir for `tui_run_test` (re-locked helper).
    async fn snapshot_dir(&self) -> PathBuf {
        self.state.lock().await.root.join("tests/snapshots")
    }
}

/// Current view for assertions (exit state from a non-blocking poll).
///
/// v1 limitation: only clean exits read as code 0; nonzero exits surface as
/// `crashed` (numeric codes arrive with richer process APIs later).
fn live_view(
    session: &mut tui_lab_core::LiveSession,
    text: &str,
) -> tui_lab_assertions::ScreenView {
    let (exit_code, crashed) = match session.pty.try_wait() {
        Ok(Some(status)) if status.success() => (Some(0), false),
        Ok(Some(_)) => (None, true),
        _ => (None, false),
    };
    tui_lab_assertions::ScreenView {
        text: text.to_string(),
        cursor: session.emu.cursor(),
        screen_changed: true,
        exit_code,
        crashed,
    }
}
