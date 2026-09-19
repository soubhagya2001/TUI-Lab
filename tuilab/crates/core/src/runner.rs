//! Sequential suite runner: the `docs/02` §2.5 pipeline (docs/07 `run`).
//!
//! One session per suite in Phase 3 (parallel fan-out arrives in v2).
//! Step failures become [`SuiteResult`] data (CLI exit 1); only
//! infrastructure breakdowns become [`CoreError`] (CLI exits 2–4).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tui_lab_assertions::{evaluate, Condition, ScreenView};
use tui_lab_input::{encode_key, encode_text};
use tui_lab_protocol::{Step, SuiteAssertion, TestFile};
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

    let all_phases = [&file.setup[..], &file.steps[..]];
    let mut aborted = false;
    for steps in all_phases {
        for step in steps {
            if failure.is_some() || aborted {
                break;
            }
            ctx.step_index = results.len();
            let step_started = Instant::now();
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
            let passed = outcome.passed;
            results.push(StepResult {
                index: ctx.step_index,
                kind: describe(step),
                passed,
                detail: outcome.detail.clone(),
                duration_ms: step_started.elapsed().as_millis() as u64,
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
        results.push(StepResult {
            index: ctx.step_index,
            kind: format!("cleanup: {}", describe(step)),
            passed: outcome.passed,
            detail: outcome.detail,
            duration_ms: step_started.elapsed().as_millis() as u64,
        });
        if !poll_hook(opts, &results) {
            break;
        }
    }

    let status = pty
        .close(None, opts.close_grace)
        .map_err(|e| CoreError::Pty(e.to_string()))?;
    let mut passed = failure.is_none();
    for assertion in &file.assertions {
        match assertion {
            SuiteAssertion::ExitCode(expected) => {
                let ok = if *expected == 0 {
                    status.success()
                } else {
                    !status.success()
                };
                if !ok {
                    passed = false;
                    failure = failure.or(Some(FailureInfo {
                        step_index: results.len(),
                        step: format!("assert exit_code {expected}"),
                        expected: format!("exit code {expected}"),
                        actual: format!("exit success={}", status.success()),
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

    Ok(SuiteResult {
        schema: TestFile::schema_id().to_string(),
        suite: file.name.clone(),
        passed,
        exit_success: Some(status.success()),
        exit_signal: status.signal().map(str::to_string),
        duration_ms: ctx.elapsed_ms(),
        steps: results,
        failure,
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
fn pump(session: &mut Session<'_>) -> String {
    let chunk = session.pty.poll(Duration::from_millis(100));
    session.emu.feed(&chunk);
    session.emu.text()
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
            session
                .pty
                .resize(to.width, to.height)
                .map_err(|e| CoreError::Pty(e.to_string()))?;
            session.emu.resize(to.width as usize, to.height as usize);
            session
                .ctx
                .input_history
                .push(format!("resize {}x{}", to.width, to.height));
            StepOutcome {
                passed: true,
                detail: format!("resized to {}x{}", to.width, to.height),
            }
        }
        Step::AssertText(assertion) | Step::Expect(assertion) => {
            let screen = pump(session);
            let view = ScreenView::live(&screen, session.emu.cursor(), true);
            let mut failures = Vec::new();
            if let Some(needle) = &assertion.contains {
                let verdict = evaluate(&Condition::TextVisible(needle.clone()), &view);
                if !verdict.passed {
                    failures.push(verdict.detail);
                }
            }
            if let Some(needle) = &assertion.not_contains {
                let verdict = evaluate(&Condition::TextNotVisible(needle.clone()), &view);
                if !verdict.passed {
                    failures.push(verdict.detail);
                }
            }
            if let Some(pattern) = &assertion.regex {
                let verdict = evaluate(&Condition::TextRegex(pattern.clone()), &view);
                if !verdict.passed {
                    failures.push(verdict.detail);
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
        Step::AssertRegion(region) => {
            let screen = pump(session);
            let area: String = screen
                .lines()
                .skip(region.y as usize)
                .take(region.height as usize)
                .map(|line| {
                    line.chars()
                        .skip(region.x as usize)
                        .take(region.width as usize)
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
                .join("\n");
            StepOutcome {
                passed: area.contains(&region.contains),
                detail: if area.contains(&region.contains) {
                    format!("region holds {:?}", region.contains)
                } else {
                    format!("region missing {:?}", region.contains)
                },
            }
        }
        Step::Snapshot(take) | Step::Screenshot(take) => {
            snapshot_step(session, take.name.clone(), take.mask.clone())?
        }
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
fn snapshot_step(
    session: &mut Session<'_>,
    name: String,
    mask: Vec<String>,
) -> Result<StepOutcome> {
    let screen = pump(session);
    let masks = compile_masks(&mask).map_err(|e| CoreError::Message(format!("{name}: {e}")))?;
    let dir = session.opts.snapshot_dir.join(&session.ctx.suite);
    let golden = dir.join(format!("{name}.txt"));
    if !Path::new(&golden).exists() {
        let new_path = dir.join(format!("{name}.new"));
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
