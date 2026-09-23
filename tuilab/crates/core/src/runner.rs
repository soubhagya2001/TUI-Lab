//! Sequential suite runner: the `docs/02` §2.5 pipeline (docs/07 `run`).
//!
//! One session per suite in Phase 3 (parallel fan-out arrives in v2).
//! Step failures become [`SuiteResult`] data (CLI exit 1); only
//! infrastructure breakdowns become [`CoreError`] (CLI exits 2–4).

use std::path::PathBuf;
use std::time::{Duration, Instant};

use tui_lab_assertions::{evaluate, Condition, ScreenView};
use tui_lab_input::{encode_key, encode_text};
use tui_lab_protocol::{Step, SuiteAssertion, TestFile, TextAssertion};
use tui_lab_pty::{PtySession, SpawnOptions};
use tui_lab_runtime::wait_for_text;
use tui_lab_snapshots::{compile_masks, load_text};
use tui_lab_terminal::Emulator;

use crate::constants::{DEFAULT_POLL_MS, DEFAULT_TIMEOUT_MS};
use crate::context::TestContext;
use crate::error::{CoreError, Result};
use crate::result::{FailureInfo, StepResult, SuiteResult, TerminalInfo};

/// Runner tuning (overridable by future CLI flags).
#[derive(Clone)]
pub struct RunOptions {
    /// Default `wait_for_text` timeout.
    pub wait_default: Duration,
    /// Screen poll interval.
    pub poll: Duration,
    /// Grace for [`PtySession::close`].
    pub close_grace: Option<Duration>,
    /// Directory holding golden snapshots.
    pub snapshot_dir: PathBuf,
    /// `$TERM` advertised (recorded in results).
    pub term: String,
    /// Step hook: called after every executed step (main and cleanup).
    /// Return false to abort remaining main steps (cleanup still runs).
    /// Core never reads terminals — blocking/interactive behavior belongs
    /// to the hook implementation (e.g. the CLI `--step` pause).
    pub step_hook: Option<StepHook>,
}

/// Per-step callback; false aborts the remaining main steps.
pub type StepHook = std::sync::Arc<dyn Fn(&StepResult) -> bool + Send + Sync>;

impl std::fmt::Debug for RunOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunOptions")
            .field("wait_default", &self.wait_default)
            .field("poll", &self.poll)
            .field("close_grace", &self.close_grace)
            .field("snapshot_dir", &self.snapshot_dir)
            .field("term", &self.term)
            .field("step_hook", &self.step_hook.is_some())
            .finish()
    }
}

impl Default for RunOptions {
    fn default() -> Self {
        Self {
            wait_default: Duration::from_millis(DEFAULT_TIMEOUT_MS),
            poll: Duration::from_millis(DEFAULT_POLL_MS),
            close_grace: None,
            snapshot_dir: PathBuf::from("tests/snapshots"),
            term: tui_lab_pty::utils::term_for(None).to_string(),
            step_hook: None,
        }
    }
}

/// Consult the step hook after a pushed result; false aborts the loop.
fn poll_hook(opts: &RunOptions, results: &[StepResult]) -> bool {
    match &opts.step_hook {
        Some(hook) => hook(&results[results.len() - 1]),
        None => true,
    }
}

/// Outcome of one step: pass, or failure evidence to record.
struct StepOutcome {
    passed: bool,
    detail: String,
}

/// Live handles shared across steps.
struct Session<'a> {
    ctx: &'a mut TestContext,
    pty: &'a mut PtySession,
    emu: &'a mut Emulator,
    opts: &'a RunOptions,
}

/// Execute a parsed suite file end to end.
///
/// C5: enforces the suite `terminal.timeout` bound via
/// [`tui_lab_runtime::run_with_timeout`] — its first caller. Without a
/// bound this is exactly [`run_file`].
pub async fn run_file_bounded(file: &TestFile, opts: &RunOptions) -> Result<SuiteResult> {
    match file.terminal.timeout {
        Some(limit) => {
            let what = format!("suite {:?}", file.name);
            match tui_lab_runtime::run_with_timeout(run_file(file, opts), limit, &what).await {
                Ok(inner) => inner,
                Err(e) => Err(CoreError::Timeout(e.to_string())),
            }
        }
        None => run_file(file, opts).await,
    }
}

/// Execute a parsed suite file end to end.
pub async fn run_file(file: &TestFile, opts: &RunOptions) -> Result<SuiteResult> {
    let mut ctx = TestContext::new("sess_001", &file.name);
    let mut results = Vec::new();
    let mut failure: Option<FailureInfo> = None;

    let spawn = SpawnOptions {
        command: file.application.command.clone(),
        args: file.application.args.clone(),
        cwd: file.application.cwd.clone().map(PathBuf::from),
        env: file
            .environment
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
        cols: file.terminal.width,
        rows: file.terminal.height,
    };
    let mut pty = PtySession::spawn(&spawn).map_err(|e| CoreError::Launch(e.to_string()))?;
    let mut emu = Emulator::new(
        file.terminal.width as usize,
        file.terminal.height as usize,
        pty.writer_sink(),
    );

    // D2: startup budget measures spawn-to-first-content. Skipped entirely
    // when unset (zero behavior change for unbudgeted suites).
    if let Some(budget) = file.budgets.startup {
        let boot = Instant::now();
        let content = loop {
            let chunk = pty.poll(Duration::from_millis(50));
            emu.feed(&chunk);
            if !emu.text().trim().is_empty() {
                break true;
            }
            if boot.elapsed() >= budget {
                break false;
            }
        };
        if !content {
            return Ok(SuiteResult {
                schema: TestFile::schema_id().to_string(),
                suite: file.name.clone(),
                passed: false,
                skipped: false,
                attempts: 1,
                exit_success: None,
                exit_signal: None,
                exit_code: None,
                duration_ms: ctx.elapsed_ms(),
                steps: Vec::new(),
                failure: Some(FailureInfo {
                    step_index: 0,
                    step: "startup".to_string(),
                    expected: format!("content within {budget:?}"),
                    actual: "blank screen".to_string(),
                    last_screen: emu.text(),
                    input_history: Vec::new(),
                }),
                terminal: TerminalInfo {
                    width: file.terminal.width,
                    height: file.terminal.height,
                    term: opts.term.clone(),
                },
                trace: Vec::new(),
                trace_truncated: false,
                attachments: Vec::new(),
            });
        }
    }

    let all_phases = [&file.setup[..], &file.steps[..]];
    let mut aborted = false;
    for steps in all_phases {
        for step in steps {
            if failure.is_some() || aborted {
                break;
            }
            ctx.step_index = results.len();
            let step_started = Instant::now();
            let step_started_ms = step_started.duration_since(ctx.started_at).as_millis() as u64;
            let outcome = run_step(
                &mut Session {
                    ctx: &mut ctx,
                    pty: &mut pty,
                    emu: &mut emu,
                    opts,
                },
                step,
            )
            .await?;
            let mut outcome = outcome;
            // D2: step budgets fail slow-but-passing steps (failures keep
            // their original, more informative detail).
            if let Some(budget) = file.budgets.step {
                let took = step_started.elapsed();
                if outcome.passed && took > budget {
                    outcome = StepOutcome {
                        passed: false,
                        detail: format!("budget exceeded: step took {took:?} (budget {budget:?})"),
                    };
                }
            }
            let passed = outcome.passed;
            results.push(StepResult {
                index: ctx.step_index,
                kind: describe(step),
                passed,
                detail: outcome.detail.clone(),
                duration_ms: step_started.elapsed().as_millis() as u64,
                started_ms: step_started_ms,
            });
            if !passed {
                failure = Some(FailureInfo {
                    step_index: ctx.step_index,
                    step: describe(step),
                    expected: outcome.detail.clone(),
                    actual: emu.text(),
                    last_screen: emu.text(),
                    input_history: ctx.input_history.clone(),
                });
            }
            if !poll_hook(opts, &results) {
                aborted = true;
            }
        }
        if aborted {
            break;
        }
    }

    // Cleanup always runs, even after failure.
    for step in &file.cleanup {
        ctx.step_index = results.len();
        let step_started = Instant::now();
        let step_started_ms = step_started.duration_since(ctx.started_at).as_millis() as u64;
        let outcome = run_step(
            &mut Session {
                ctx: &mut ctx,
                pty: &mut pty,
                emu: &mut emu,
                opts,
            },
            step,
        )
        .await?;
        let mut outcome = outcome;
        // D2: step budgets apply to cleanup steps too.
        if let Some(budget) = file.budgets.step {
            let took = step_started.elapsed();
            if outcome.passed && took > budget {
                outcome = StepOutcome {
                    passed: false,
                    detail: format!("budget exceeded: step took {took:?} (budget {budget:?})"),
                };
            }
        }
        results.push(StepResult {
            index: ctx.step_index,
            kind: format!("cleanup: {}", describe(step)),
            passed: outcome.passed,
            detail: outcome.detail,
            duration_ms: step_started.elapsed().as_millis() as u64,
            started_ms: step_started_ms,
        });
        if !poll_hook(opts, &results) {
            break;
        }
    }

    let status = pty
        .close(None, opts.close_grace)
        .map_err(|e| CoreError::Pty(e.to_string()))?;
    let mut passed = failure.is_none();
    let exit_code = status.exit_code() as i32;
    for assertion in &file.assertions {
        match assertion {
            SuiteAssertion::ExitCode(expected) => {
                let ok = exit_code == *expected;
                if !ok {
                    passed = false;
                    failure = failure.or(Some(FailureInfo {
                        step_index: results.len(),
                        step: format!("assert exit_code {expected}"),
                        expected: format!("exit code {expected}"),
                        actual: format!("exit code {exit_code}"),
                        last_screen: emu.text(),
                        input_history: ctx.input_history.clone(),
                    }));
                }
            }
            SuiteAssertion::ProcessNotCrashed(expected) => {
                // Conservative v1: any failure to exit cleanly counts as a crash.
                let crashed = !status.success();
                if crashed == *expected {
                    passed = false;
                    failure = failure.or(Some(FailureInfo {
                        step_index: results.len(),
                        step: "assert not crashed".to_string(),
                        expected: format!("crashed == {expected}"),
                        actual: format!("crashed == {crashed}"),
                        last_screen: emu.text(),
                        input_history: ctx.input_history.clone(),
                    }));
                }
            }
        }
    }

    // D2: suite budget is a test failure with evidence (not an infra
    // error like `terminal.timeout`). Only applies while still passing —
    // the first failure already tells the story.
    if let Some(budget) = file.budgets.suite {
        let took = ctx.started_at.elapsed();
        if passed && took > budget {
            passed = false;
            failure = Some(FailureInfo {
                step_index: results.len(),
                step: "budget".to_string(),
                expected: format!("run within {budget:?}"),
                actual: format!("run took {took:?}"),
                last_screen: emu.text(),
                input_history: ctx.input_history.clone(),
            });
        }
    }

    Ok(SuiteResult {
        schema: TestFile::schema_id().to_string(),
        suite: file.name.clone(),
        passed,
        skipped: false,
        attempts: 1,
        exit_success: Some(status.success()),
        exit_signal: status.signal().map(str::to_string),
        exit_code: Some(exit_code),
        duration_ms: ctx.elapsed_ms(),
        steps: results,
        failure,
        trace: std::mem::take(&mut ctx.trace),
        trace_truncated: ctx.trace_truncated,
        attachments: Vec::new(),
        terminal: TerminalInfo {
            width: file.terminal.width,
            height: file.terminal.height,
            term: opts.term.clone(),
        },
    })
}

/// One-line human description for reports.
fn describe(step: &Step) -> String {
    match step {
        Step::Press(key) => format!("press {key}"),
        Step::Type(text) => format!("type {text:?}"),
        Step::WaitForText(wait) => format!("wait_for_text {:?}", wait.text),
        Step::Sleep(sleep) => format!("sleep {:?}", sleep.0),
        Step::Resize(to) => format!("resize {}x{}", to.width, to.height),
        Step::AssertText(assertion) => format!("assert_text {assertion:?}"),
        Step::Expect(assertion) => format!("expect {assertion:?}"),
        Step::AssertRegion(region) => format!("assert_region {region:?}"),
        Step::Snapshot(take) | Step::Screenshot(take) => {
            format!("snapshot {:?}", take.name)
        }
        Step::WaitForExit(wait) => format!("wait_for_exit {:?}", wait.timeout),
    }
}

/// Feed one PTY chunk into the grid; return the fresh screen text.
///
/// Records the screen as the `screen_changed` baseline for the next pump,
/// and captures raw bytes (capped) for trace replay.
fn pump(session: &mut Session<'_>) -> String {
    let chunk = session.pty.poll(Duration::from_millis(100));
    session.emu.feed(&chunk);
    let text = session.emu.text();
    session.ctx.prev_screen = text.clone();
    if !chunk.is_empty() && session.ctx.trace_bytes < crate::constants::TRACE_MAX_BYTES {
        session.ctx.trace_bytes += chunk.len();
        session.ctx.trace.push(crate::result::TraceChunk {
            at_ms: session.ctx.elapsed_ms(),
            bytes: chunk,
        });
    } else if !chunk.is_empty() {
        session.ctx.trace_truncated = true;
    }
    text
}

/// Evaluate every check in one assertion against one screen (C3: the full
/// taxonomy — exact text, cursor, exit code, crash state, screen change —
/// is reachable from YAML, not just contains/not_contains/regex).
fn evaluate_assertion(
    assertion: &TextAssertion,
    screen: &str,
    cursor: (usize, usize),
    changed: bool,
    exit: (Option<i32>, bool),
    compiled: Option<&regex::Regex>,
) -> StepOutcome {
    let mut conditions = Vec::new();
    if let Some(needle) = &assertion.contains {
        conditions.push(Condition::TextVisible(needle.clone()));
    }
    if let Some(needle) = &assertion.not_contains {
        conditions.push(Condition::TextNotVisible(needle.clone()));
    }
    if let Some(exact) = &assertion.exact_text {
        conditions.push(Condition::ExactText(exact.clone()));
    }
    if let Some(cursor_pos) = &assertion.cursor {
        conditions.push(Condition::CursorPosition {
            row: cursor_pos.row,
            col: cursor_pos.col,
        });
    }
    if let Some(code) = assertion.exit_code {
        conditions.push(Condition::ExitCode(code));
    }
    if let Some(expected) = assertion.screen_changed {
        conditions.push(Condition::ScreenChanged(expected));
    }
    if let Some(healthy) = assertion.not_crashed {
        conditions.push(if healthy {
            Condition::NotCrashed
        } else {
            Condition::Crashed
        });
    }
    let view = ScreenView {
        text: screen.to_string(),
        cursor,
        screen_changed: changed,
        exit_code: exit.0,
        crashed: exit.1,
    };
    let mut failures = Vec::new();
    for condition in &conditions {
        let verdict = evaluate(condition, &view);
        if !verdict.passed {
            failures.push(verdict.detail);
        }
    }
    // R1: regex runs against the loop-precompiled pattern — never
    // recompiled per tick. Invalid patterns never reach here:
    // `assert_poll` fails them fast up front.
    if let Some(pattern) = &assertion.regex {
        let matched = compiled.is_some_and(|compiled| compiled.is_match(screen));
        if !matched {
            failures.push(format!("regex did not match: {pattern:?}"));
        }
    }
    StepOutcome {
        passed: failures.is_empty(),
        detail: if failures.is_empty() {
            "assertions held".to_string()
        } else {
            failures.join("; ")
        },
    }
}

/// C2: poll an assertion until it holds or `timeout` elapses. With no
/// timeout the step keeps the old single-attempt behavior.
async fn assert_poll(session: &mut Session<'_>, assertion: &TextAssertion) -> Result<StepOutcome> {
    let poll = session.opts.poll;
    let start = Instant::now();
    // R1: compile once; an invalid pattern fails fast (same message shape
    // as `evaluate`, without polling the timeout blind).
    let compiled = match assertion.regex.as_deref() {
        Some(pattern) => match regex::Regex::new(pattern) {
            Ok(compiled) => Some(compiled),
            Err(e) => {
                return Ok(StepOutcome {
                    passed: false,
                    detail: format!("invalid regex {pattern:?}: {e}"),
                })
            }
        },
        None => None,
    };
    // Baseline for `screen_changed`: the screen as last observed before
    // this step's first pump.
    let baseline = session.ctx.prev_screen.clone();
    loop {
        let screen = pump(session);
        let changed = screen != baseline;
        let status = session
            .pty
            .try_wait()
            .map_err(|e| CoreError::Pty(e.to_string()))?;
        let exit = tui_lab_pty::utils::exit_view(status.as_ref());
        let outcome = evaluate_assertion(
            assertion,
            &screen,
            session.emu.cursor(),
            changed,
            exit,
            compiled.as_ref(),
        );
        if outcome.passed {
            return Ok(outcome);
        }
        match assertion.timeout {
            Some(limit) if start.elapsed() < limit => tokio::time::sleep(poll).await,
            _ => return Ok(outcome),
        }
    }
}

async fn run_step(session: &mut Session<'_>, step: &Step) -> Result<StepOutcome> {
    let outcome = match step {
        Step::Press(key) => {
            let bytes = encode_key(key).map_err(|e| CoreError::Message(format!("{key:?}: {e}")))?;
            session
                .pty
                .write_all(&bytes)
                .map_err(|e| CoreError::Pty(e.to_string()))?;
            session.ctx.input_history.push(format!("press {key}"));
            StepOutcome {
                passed: true,
                detail: format!("sent {key}"),
            }
        }
        Step::Type(text) => {
            session
                .pty
                .write_all(&encode_text(text))
                .map_err(|e| CoreError::Pty(e.to_string()))?;
            session
                .ctx
                .input_history
                .push(format!("type[len={}]", text.len()));
            StepOutcome {
                passed: true,
                detail: format!("typed {} chars", text.len()),
            }
        }
        Step::WaitForText(wait) => {
            let timeout = wait.timeout.unwrap_or(session.opts.wait_default);
            let poll = session.opts.poll;
            let seen = {
                let outcome =
                    wait_for_text(|| pump(session), &wait.text, wait.regex, timeout, poll).await;
                outcome
            };
            session
                .ctx
                .input_history
                .push(format!("wait {:?}", wait.text));
            StepOutcome {
                passed: seen.found,
                detail: if seen.found {
                    format!("found {:?}", wait.text)
                } else {
                    format!("timeout waiting for {:?}", wait.text)
                },
            }
        }
        Step::Sleep(sleep) => {
            tokio::time::sleep(sleep.0).await;
            StepOutcome {
                passed: true,
                detail: format!("slept {:?}", sleep.0),
            }
        }
        Step::Resize(to) => {
            // Shared helper validates, clamps, and resizes PTY + grid.
            let (width, height) = crate::sessions::resize_live(
                &mut *session.pty,
                &mut *session.emu,
                to.width,
                to.height,
            )?;
            session
                .ctx
                .input_history
                .push(format!("resize {width}x{height}"));
            StepOutcome {
                passed: true,
                detail: if (width, height) == (to.width, to.height) {
                    format!("resized to {width}x{height}")
                } else {
                    format!(
                        "resized to {width}x{height} (clamped from {}x{})",
                        to.width, to.height
                    )
                },
            }
        }
        Step::AssertText(assertion) | Step::Expect(assertion) => {
            assert_poll(session, assertion).await?
        }
        Step::AssertRegion(region) => {
            // R3: cell-scoped extraction with named-bounds errors (OOB is a
            // step failure with evidence, not an infra abort).
            pump(session);
            let area = match session.emu.region_text(
                region.x as usize,
                region.y as usize,
                region.width as usize,
                region.height as usize,
            ) {
                Ok(area) => area,
                Err(bounds) => {
                    return Ok(StepOutcome {
                        passed: false,
                        detail: format!("assert_region out of bounds: {bounds}"),
                    })
                }
            };
            StepOutcome {
                passed: area.contains(&region.contains),
                detail: if area.contains(&region.contains) {
                    format!("region holds {:?}", region.contains)
                } else {
                    format!("region missing {:?}", region.contains)
                },
            }
        }
        Step::Snapshot(take) | Step::Screenshot(take) => snapshot_step(session, take)?,
        Step::WaitForExit(wait) => {
            let start = Instant::now();
            let exited = loop {
                if session
                    .pty
                    .try_wait()
                    .map_err(|e| CoreError::Pty(e.to_string()))?
                    .is_some()
                {
                    break true;
                }
                if start.elapsed() >= wait.timeout {
                    break false;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            };
            StepOutcome {
                passed: exited,
                detail: if exited {
                    "process exited".to_string()
                } else {
                    format!("no exit within {:?}", wait.timeout)
                },
            }
        }
    };
    Ok(outcome)
}

/// Capture the screen and compare against the golden (first run writes `.new`
/// and fails so goldens are always approved explicitly).
///
/// C4: goldens live at the size-scoped store path
/// (`<suite>/<name>/<WxH>.txt`), never the flat `<name>.txt` — different
/// terminal sizes must not collide.
///
/// D1: `styled` compares per-cell attributes (colors included) via
/// `<WxH>.cells.json`; `graphics` compares captured Sixel payloads via
/// `<WxH>.sixel.json`. Masks apply to text snapshots only.
fn snapshot_step(
    session: &mut Session<'_>,
    take: &tui_lab_protocol::SnapshotTake,
) -> Result<StepOutcome> {
    let name = take.name.clone();
    let screen = pump(session);
    let dir = session.opts.snapshot_dir.join(&session.ctx.suite);
    let (cols, rows) = session.emu.dims();
    let (width, height) = (cols as u16, rows as u16);
    if take.graphics {
        return graphics_snapshot(session, &dir, &name, width, height, &take.mask);
    }
    if take.styled {
        return styled_snapshot(session, &dir, &name, width, height, &take.mask);
    }
    let masks =
        compile_masks(&take.mask).map_err(|e| CoreError::Message(format!("{name}: {e}")))?;
    let golden = tui_lab_snapshots::text_golden_path(&dir, &name, width, height);
    if !golden.exists() {
        let new_path = golden.with_extension("new");
        if let Some(parent) = new_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| CoreError::Message(e.to_string()))?;
        }
        std::fs::write(&new_path, &screen).map_err(|e| CoreError::Message(e.to_string()))?;
        return Ok(StepOutcome {
            passed: false,
            detail: format!(
                "new golden written to {} — approve and re-run",
                new_path.display()
            ),
        });
    }
    let expected = load_text(&golden).map_err(|e| CoreError::Message(format!("{name}: {e}")))?;
    let outcome = tui_lab_snapshots::compare_text(&expected, &screen, &masks);
    Ok(StepOutcome {
        passed: outcome.equal,
        detail: if outcome.equal {
            format!("snapshot {name:?} matched")
        } else {
            format!("snapshot {name:?} differs:\n{}", outcome.diff)
        },
    })
}

/// Styled snapshot: per-cell attributes (colors included) via
/// `<WxH>.cells.json`. Masks apply to text snapshots only.
fn styled_snapshot(
    session: &mut Session<'_>,
    dir: &std::path::Path,
    name: &str,
    width: u16,
    height: u16,
    mask: &[String],
) -> Result<StepOutcome> {
    if !mask.is_empty() {
        return Err(CoreError::Message(format!(
            "{name}: masks apply to text snapshots only"
        )));
    }
    let cells: Vec<tui_lab_snapshots::CellData> = session
        .emu
        .cells()
        .into_iter()
        .map(tui_lab_snapshots::CellData::from)
        .collect();
    let snapshot = tui_lab_snapshots::CellSnapshot {
        width,
        height,
        cells,
    };
    let golden = tui_lab_snapshots::cells_golden_path(dir, name, width, height);
    if !golden.exists() {
        let json = tui_lab_snapshots::cells_to_json(&snapshot)
            .map_err(|e| CoreError::Message(format!("{name}: {e}")))?;
        let new_path = golden.with_extension("new");
        if let Some(parent) = new_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| CoreError::Message(e.to_string()))?;
        }
        std::fs::write(&new_path, &json).map_err(|e| CoreError::Message(e.to_string()))?;
        return Ok(StepOutcome {
            passed: false,
            detail: format!(
                "new golden written to {} — approve and re-run",
                new_path.display()
            ),
        });
    }
    let expected = tui_lab_snapshots::load_cells(&golden)
        .map_err(|e| CoreError::Message(format!("{name}: {e}")))?;
    let actual = tui_lab_snapshots::CellSnapshot {
        width,
        height,
        cells: snapshot.cells,
    };
    let outcome = tui_lab_snapshots::compare_cells(&expected, &actual);
    Ok(StepOutcome {
        passed: outcome.equal,
        detail: if outcome.equal {
            format!("styled snapshot {name:?} matched")
        } else {
            format!("styled snapshot {name:?} differs:\n{}", outcome.diff)
        },
    })
}

/// Sixel snapshot: captured graphics payloads via `<WxH>.sixel.json`.
/// Exact normalized equality — no tolerance theater (font-free rendering
/// would flake across machines). Masks apply to text snapshots only.
fn graphics_snapshot(
    session: &mut Session<'_>,
    dir: &std::path::Path,
    name: &str,
    width: u16,
    height: u16,
    mask: &[String],
) -> Result<StepOutcome> {
    if !mask.is_empty() {
        return Err(CoreError::Message(format!(
            "{name}: masks apply to text snapshots only"
        )));
    }
    pump(session);
    let sixels: Vec<String> = session
        .emu
        .sixels()
        .iter()
        .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
        .collect();
    let golden = tui_lab_snapshots::sixel_golden_path(dir, name, width, height);
    if !golden.exists() {
        let json = serde_json::to_string_pretty(&sixels)
            .map_err(|e| CoreError::Message(format!("{name}: {e}")))?;
        let new_path = golden.with_extension("new");
        if let Some(parent) = new_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| CoreError::Message(e.to_string()))?;
        }
        std::fs::write(&new_path, &json).map_err(|e| CoreError::Message(e.to_string()))?;
        return Ok(StepOutcome {
            passed: false,
            detail: format!(
                "new golden written to {} — approve and re-run",
                new_path.display()
            ),
        });
    }
    let expected = tui_lab_snapshots::load_sixels(&golden)
        .map_err(|e| CoreError::Message(format!("{name}: {e}")))?;
    let outcome = tui_lab_snapshots::compare_sixels(&expected, &sixels);
    Ok(StepOutcome {
        passed: outcome.equal,
        detail: if outcome.equal {
            format!("graphics snapshot {name:?} matched")
        } else {
            format!("graphics snapshot {name:?} differs:\n{}", outcome.diff)
        },
    })
}
