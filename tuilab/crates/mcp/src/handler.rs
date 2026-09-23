//! MCP handler: the 10 `tui_*` tools over shared core state (docs/08).
//!
//! Modes: A (interactive launch → inspect → act loop) and B (`tui_run_test`
//! full suites). Every input tool's description ends with the sync rule:
//! follow it with `tui_wait_for_text`, never assert on a stale screen.

use std::path::PathBuf;
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
use tui_lab_runtime::matches as screen_matches;

use crate::constants::{MAX_SESSIONS, SESSION_IDLE_SECS};
use crate::security::{jail, jail_file, Allowlist, AuditConfig};
use crate::tools::{
    AssertOut, AssertParams, CloseOut, CloseParams, CursorPos, LaunchOut, LaunchParams, PressOut,
    PressParams, ResizeOut, ResizeParams, RunFailure, RunTestOut, RunTestParams, ScreenOut,
    ScreenParams, SnapshotOut, SnapshotParams, TypeOut, TypeParams, WaitOut, WaitParams,
};

/// Reap sessions idle longer than this on every launch.
const IDLE_REAP_SECS: u64 = SESSION_IDLE_SECS;

/// Short pump budget after single inputs.
const INPUT_SETTLE_MS: u64 = 300;

/// Shared server state behind one async mutex.
///
/// Locking discipline (S1): the mutex guards map/accounting only. Every tool
/// takes it for short synchronous critical sections and NEVER holds it
/// across `.await` — `tui_wait_for_text` re-locks per poll tick so one slow
/// wait cannot serialize the other sessions.
pub struct HandlerState {
    /// Live PTY sessions (core-owned).
    pub registry: SessionRegistry,
    /// Project root (cwd jail + suite resolution).
    pub root: PathBuf,
    /// Launch allowlist.
    pub allow: Allowlist,
    /// Idle timeout for opportunistic GC (R10).
    pub idle: Duration,
    /// Call audit policy (opt-in JSONL).
    pub audit: AuditConfig,
}

/// `tuilab-mcp` request handler.
#[derive(Clone)]
pub struct TuiLabHandler {
    state: Arc<tokio::sync::Mutex<HandlerState>>,
    tool_router: ToolRouter<Self>,
}

impl TuiLabHandler {
    /// Build a handler rooted at `root` with the given allowlist.
    pub fn new(root: PathBuf, allow: Allowlist, audit: AuditConfig) -> Self {
        Self::with_idle(root, allow, audit, Duration::from_secs(IDLE_REAP_SECS))
    }

    /// Build a handler with an explicit idle timeout (tests use short ones).
    pub fn with_idle(root: PathBuf, allow: Allowlist, audit: AuditConfig, idle: Duration) -> Self {
        let state = Arc::new(tokio::sync::Mutex::new(HandlerState {
            registry: SessionRegistry::with_cap(MAX_SESSIONS),
            root,
            allow,
            idle,
            audit,
        }));
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }

    /// Opportunistic GC (R10): reap idle sessions on every tool entry, so
    /// abandoned sessions die without waiting for the next launch. Takes
    /// the lock only for the fast synchronous sweep.
    async fn reap(&self) {
        let mut state = self.state.lock().await;
        let idle = state.idle;
        state.registry.reap_idle(idle);
    }

    /// Append one audit line when auditing is enabled (opt-in JSONL).
    ///
    /// Never fails the tool: sink errors trace and drop. `args` carries the
    /// caller-supplied redaction (e.g. `tui_type` secrets arrive redacted).
    async fn audit(
        &self,
        tool: &str,
        session: Option<&str>,
        ok: bool,
        detail: &str,
        args: Option<serde_json::Value>,
        elapsed: std::time::Duration,
    ) {
        use crate::security::AuditField;
        let cfg = { self.state.lock().await.audit.clone() };
        if !cfg.enabled {
            return;
        }
        let mut record = serde_json::Map::with_capacity(cfg.fields.len());
        for field in &cfg.fields {
            match field {
                AuditField::Timestamp => {
                    let millis = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0);
                    record.insert("timestamp".to_string(), millis.into());
                }
                AuditField::Tool => {
                    record.insert("tool".to_string(), tool.into());
                }
                AuditField::Session => {
                    record.insert("session_id".to_string(), session.map(str::to_string).into());
                }
                AuditField::Result => {
                    record.insert(
                        "result".to_string(),
                        if ok {
                            "ok".into()
                        } else {
                            detail.to_string().into()
                        },
                    );
                }
                AuditField::Args => {
                    record.insert(
                        "args".to_string(),
                        args.clone().unwrap_or(serde_json::Value::Null),
                    );
                }
                AuditField::Elapsed => {
                    record.insert(
                        "elapsed_ms".to_string(),
                        (elapsed.as_millis() as u64).into(),
                    );
                }
            }
        }
        let line = serde_json::Value::Object(record).to_string();
        let path = cfg.path.clone();
        let write = tokio::task::spawn_blocking(move || -> Result<(), String> {
            use std::io::Write as _;
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("audit mkdir {}: {e}", parent.display()))?;
                }
            }
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .map_err(|e| format!("audit open {}: {e}", path.display()))?;
            let mut buf = line;
            buf.push('\n');
            file.write_all(buf.as_bytes())
                .map_err(|e| format!("audit write {}: {e}", path.display()))
        })
        .await;
        match write {
            Ok(Ok(())) => {}
            Ok(Err(e)) => tracing::warn!("audit sink failed: {e}"),
            Err(e) => tracing::warn!("audit task failed: {e}"),
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
        let started = std::time::Instant::now();
        let args = serde_json::json!({"command": params.0.command, "args": params.0.args});
        let outcome = self.launch_inner(params.0).await;
        let session = outcome.as_ref().ok().map(|ok| ok.0.session_id.as_str());
        let detail = outcome.as_ref().err().cloned().unwrap_or_default();
        self.audit(
            "tui_launch",
            session,
            outcome.is_ok(),
            &detail,
            Some(args),
            started.elapsed(),
        )
        .await;
        outcome
    }

    async fn launch_inner(&self, params: LaunchParams) -> Result<Json<LaunchOut>, String> {
        self.reap().await;
        let mut state = self.state.lock().await;
        state
            .allow
            .check(&params.command, &params.args)
            .map_err(|e| tool_error("launch", e))?;
        let cwd = jail(&state.root, params.cwd.as_deref()).map_err(|e| tool_error("launch", e))?;
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
        let started = std::time::Instant::now();
        let session = params.0.session_id.clone();
        let args = serde_json::json!({"key": params.0.key});
        let outcome = self.press_inner(params.0).await;
        let detail = outcome.as_ref().err().cloned().unwrap_or_default();
        self.audit(
            "tui_press",
            Some(&session),
            outcome.is_ok(),
            &detail,
            Some(args),
            started.elapsed(),
        )
        .await;
        outcome
    }

    async fn press_inner(&self, params: PressParams) -> Result<Json<PressOut>, String> {
        self.reap().await;
        let mut state = self.state.lock().await;
        let session = state
            .registry
            .get_mut(&params.session_id)
            .map_err(|e| tool_error("press", e))?;
        let bytes = encode_key(&params.key)
            .map_err(|e| tool_error("press", format!("{:?}: {e}", params.key)))?;
        let before = session.emu.text();
        if let Some(ms) = params.delay_ms.filter(|ms| *ms > 0) {
            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
        }
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
        let started = std::time::Instant::now();
        let session = params.0.session_id.clone();
        // Sensitive text is redacted before it can reach the audit sink.
        let args = if params.0.sensitive {
            serde_json::json!({"text": "[redacted]", "sensitive": true})
        } else {
            serde_json::json!({"text": params.0.text, "sensitive": false})
        };
        let outcome = self.type_inner(params.0).await;
        let detail = outcome.as_ref().err().cloned().unwrap_or_default();
        self.audit(
            "tui_type",
            Some(&session),
            outcome.is_ok(),
            &detail,
            Some(args),
            started.elapsed(),
        )
        .await;
        outcome
    }

    async fn type_inner(&self, params: TypeParams) -> Result<Json<TypeOut>, String> {
        self.reap().await;
        let mut state = self.state.lock().await;
        let session = state
            .registry
            .get_mut(&params.session_id)
            .map_err(|e| tool_error("type", e))?;
        // P5-E2: optional per-character pacing (delay_ms).
        match params.delay_ms.filter(|ms| *ms > 0) {
            Some(ms) => {
                let gap = std::time::Duration::from_millis(ms);
                for (i, ch) in params.text.chars().enumerate() {
                    if i > 0 {
                        tokio::time::sleep(gap).await;
                    }
                    let mut buf = [0u8; 4];
                    let bytes = ch.encode_utf8(&mut buf).as_bytes();
                    session
                        .pty
                        .write_all(bytes)
                        .map_err(|e| tool_error("type", e))?;
                }
            }
            None => {
                session
                    .pty
                    .write_all(&encode_text(&params.text))
                    .map_err(|e| tool_error("type", e))?;
            }
        }
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
        let started = std::time::Instant::now();
        let session = params.0.session_id.clone();
        let args = serde_json::json!({"styled": params.0.styled});
        let outcome = self.screen_inner(params.0).await;
        let detail = outcome.as_ref().err().cloned().unwrap_or_default();
        self.audit(
            "tui_screen",
            Some(&session),
            outcome.is_ok(),
            &detail,
            Some(args),
            started.elapsed(),
        )
        .await;
        outcome
    }

    async fn screen_inner(&self, params: ScreenParams) -> Result<Json<ScreenOut>, String> {
        self.reap().await;
        let mut state = self.state.lock().await;
        let session = state
            .registry
            .get_mut(&params.session_id)
            .map_err(|e| tool_error("screen", e))?;
        let text = SessionRegistry::pump_once(session, Duration::from_millis(100));
        let (width, height) = session.emu.dims();
        let (row, col) = session.emu.cursor();
        let cells = params.styled.then(|| {
            session
                .emu
                .cells()
                .into_iter()
                .map(|cell| crate::tools::ScreenCell {
                    x: cell.x,
                    y: cell.y,
                    char: cell.character.to_string(),
                    fg: cell.fg,
                    bg: cell.bg,
                    bold: cell.bold,
                    underline: cell.underline,
                    reverse: cell.reverse,
                })
                .collect()
        });
        let tree = params.tree.then(|| {
            session
                .emu
                .a11y_tree()
                .into_iter()
                .map(|node| crate::tools::TreeNode {
                    role: node.role.name().to_string(),
                    name: node.name,
                    x: node.x,
                    y: node.y,
                    focused: node.focused,
                })
                .collect()
        });
        Ok(Json(ScreenOut {
            width,
            height,
            cursor: CursorPos { row, col },
            text,
            cells,
            tree,
        }))
    }

    /// Poll until text appears (or the timeout elapses). Prefer this over any
    /// fixed sleep — TUI redraw is asynchronous.
    ///
    /// Concurrency (S1): the state lock is re-acquired per poll tick and
    /// never held across the sleep, so waits on different sessions overlap
    /// instead of serializing behind one slow wait.
    #[tool(
        name = "tui_wait_for_text",
        description = "Wait until text appears on screen (polling with timeout). Use after every input instead of sleeping."
    )]
    pub async fn tui_wait_for_text(
        &self,
        params: Parameters<WaitParams>,
    ) -> Result<Json<WaitOut>, String> {
        let started = std::time::Instant::now();
        let session = params.0.session_id.clone();
        let args = serde_json::json!({
            "text": params.0.text,
            "regex": params.0.regex,
            "timeout_ms": params.0.timeout_ms,
        });
        let outcome = self.wait_inner(params.0).await;
        let detail = outcome.as_ref().err().cloned().unwrap_or_default();
        self.audit(
            "tui_wait_for_text",
            Some(&session),
            outcome.is_ok(),
            &detail,
            Some(args),
            started.elapsed(),
        )
        .await;
        outcome
    }

    async fn wait_inner(&self, params: WaitParams) -> Result<Json<WaitOut>, String> {
        // R6: an explicit zero timeout means "use the default", never
        // "fail instantly".
        let timeout =
            Duration::from_millis(tui_lab_core::utils::clamp_timeout_ms(params.timeout_ms));
        let poll = Duration::from_millis(50);
        let start = std::time::Instant::now();
        let mut last_screen = String::new();
        let mut found = false;
        self.reap().await;
        while start.elapsed() < timeout {
            // Short critical section: pump under the lock, match + sleep outside.
            // (Entry reap above covers GC; reaping per tick would churn.)
            let text = {
                let mut state = self.state.lock().await;
                let session = state
                    .registry
                    .get_mut(&params.session_id)
                    .map_err(|e| tool_error("wait", e))?;
                SessionRegistry::pump_once(session, Duration::from_millis(50))
            };
            last_screen = text;
            if screen_matches(&last_screen, &params.text, params.regex) {
                found = true;
                break;
            }
            tokio::time::sleep(poll).await;
        }
        Ok(Json(WaitOut {
            found,
            elapsed_ms: start.elapsed().as_millis() as u64,
            screen: last_screen,
        }))
    }

    /// Evaluate one assertion against the live screen. Prefer waiting first
    /// with tui_wait_for_text so the verdict reflects the latest state.
    #[tool(
        name = "tui_assert",
        description = "Assert a condition (text_visible, text_not_visible, text_regex, exact_text, cursor_position, screen_changed, exit_code, not_crashed, crashed) against the live screen."
    )]
    pub async fn tui_assert(
        &self,
        params: Parameters<AssertParams>,
    ) -> Result<Json<AssertOut>, String> {
        let started = std::time::Instant::now();
        let session = params.0.session_id.clone();
        let args = params.0.assertion.clone();
        let outcome = self.assert_inner(params.0).await;
        let detail = outcome.as_ref().err().cloned().unwrap_or_default();
        self.audit(
            "tui_assert",
            Some(&session),
            outcome.is_ok(),
            &detail,
            Some(args),
            started.elapsed(),
        )
        .await;
        outcome
    }

    async fn assert_inner(&self, params: AssertParams) -> Result<Json<AssertOut>, String> {
        self.reap().await;
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
        let started = std::time::Instant::now();
        let session = params.0.session_id.clone();
        let args = serde_json::json!({"name": params.0.name});
        let outcome = self.snapshot_inner(params.0).await;
        let detail = outcome.as_ref().err().cloned().unwrap_or_default();
        self.audit(
            "tui_snapshot",
            Some(&session),
            outcome.is_ok(),
            &detail,
            Some(args),
            started.elapsed(),
        )
        .await;
        outcome
    }

    async fn snapshot_inner(&self, params: SnapshotParams) -> Result<Json<SnapshotOut>, String> {
        self.reap().await;
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
        let started = std::time::Instant::now();
        let args = serde_json::json!({"test_file": params.0.test_file});
        let outcome = self.run_test_inner(params.0).await;
        let detail = outcome.as_ref().err().cloned().unwrap_or_default();
        self.audit(
            "tui_run_test",
            None,
            outcome.is_ok(),
            &detail,
            Some(args),
            started.elapsed(),
        )
        .await;
        outcome
    }

    async fn run_test_inner(&self, params: RunTestParams) -> Result<Json<RunTestOut>, String> {
        self.reap().await;
        let state = self.state.lock().await;
        // S3: confine the suite file under the project root (no traversal).
        let suite_path =
            jail_file(&state.root, &params.test_file).map_err(|e| tool_error("run_test", e))?;
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
        let started = std::time::Instant::now();
        let session = params.0.session_id.clone();
        let args = serde_json::json!({"quit": params.0.quit});
        let outcome = self.close_inner(params.0).await;
        let detail = outcome.as_ref().err().cloned().unwrap_or_default();
        self.audit(
            "tui_close",
            Some(&session),
            outcome.is_ok(),
            &detail,
            Some(args),
            started.elapsed(),
        )
        .await;
        outcome
    }

    async fn close_inner(&self, params: CloseParams) -> Result<Json<CloseOut>, String> {
        self.reap().await;
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
            exit_code: closed.exit_code,
            signal: closed.signal,
            detail: closed.note,
        }))
    }

    /// Resize a session's PTY + grid. Redraw is async: follow with
    /// tui_wait_for_text — never assert on a stale screen.
    #[tool(
        name = "tui_resize",
        description = "Resize a session terminal. Returns the actual dimensions after clamping. Then call tui_wait_for_text — redraw is async."
    )]
    pub async fn tui_resize(
        &self,
        params: Parameters<ResizeParams>,
    ) -> Result<Json<ResizeOut>, String> {
        let started = std::time::Instant::now();
        let session = params.0.session_id.clone();
        let args = serde_json::json!({"width": params.0.width, "height": params.0.height});
        let outcome = self.resize_inner(params.0).await;
        let detail = outcome.as_ref().err().cloned().unwrap_or_default();
        self.audit(
            "tui_resize",
            Some(&session),
            outcome.is_ok(),
            &detail,
            Some(args),
            started.elapsed(),
        )
        .await;
        outcome
    }

    async fn resize_inner(&self, params: ResizeParams) -> Result<Json<ResizeOut>, String> {
        self.reap().await;
        let mut state = self.state.lock().await;
        let (width, height) = state
            .registry
            .resize(&params.session_id, params.width, params.height)
            .map_err(|e| tool_error("resize", e))?;
        Ok(Json(ResizeOut {
            ok: true,
            width,
            height,
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
fn live_view(
    session: &mut tui_lab_core::LiveSession,
    text: &str,
) -> tui_lab_assertions::ScreenView {
    // C1: numeric codes survive (ExitCode(2) is assertable); only signal
    // deaths count as crashed. try_wait errors mean "unknown", not crashed.
    let status = session.pty.try_wait().ok().flatten();
    let (exit_code, crashed) = tui_lab_pty::utils::exit_view(status.as_ref());
    tui_lab_assertions::ScreenView {
        text: text.to_string(),
        cursor: session.emu.cursor(),
        screen_changed: true,
        exit_code,
        crashed,
        tree: session.emu.a11y_tree(),
    }
}
