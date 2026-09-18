//! Phase 2 exit gate: one YAML suite executed through every engine layer.
//!
//! Dress rehearsal for `tuilab run` — parse YAML (`protocol`), drive the
//! fixture (`pty` + `terminal` + `input`), synchronize (`runtime`),
//! assert (`assertions`), snapshot (`snapshots`). Single session only;
//! the registry, `TestContext`, and reporters arrive in Phases 3–4.

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use tui_lab_assertions::{evaluate, Condition, ScreenView};
use tui_lab_input::{encode_key, encode_text};
use tui_lab_protocol::{Step, SuiteAssertion, TestFile, TextAssertion};
use tui_lab_pty::{PtySession, SpawnOptions};
use tui_lab_runtime::wait_for_text;
use tui_lab_snapshots::{compare_text, save_text};
use tui_lab_terminal::Emulator;

const WAIT_TIMEOUT: Duration = Duration::from_secs(10);
const POLL: Duration = Duration::from_millis(50);

/// Build the fixture binary on demand; return its path.
///
/// Mirrors the helper in `runtime_smoke.rs`; both collapse into the Phase 3
/// runner's process management.
fn fixture_bin() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/ratatui-sample");
    let output = Command::new("cargo")
        .arg("build")
        .current_dir(&dir)
        .output()
        .expect("run cargo build for fixture");
    assert!(
        output.status.success(),
        "fixture build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bin = if cfg!(windows) {
        "ratatui-sample.exe"
    } else {
        "ratatui-sample"
    };
    dir.join("target").join("debug").join(bin)
}

fn suite_yaml(bin: &str) -> String {
    format!(
        r#"
schema: tui-lab/v1
name: single-session proof
application:
  command: "{bin}"
terminal:
  width: 120
  height: 40
steps:
  - wait_for_text:
      text: "TUI-LAB-SAMPLE"
  - press: DOWN
  - press: ENTER
  - wait_for_text:
      text: "Selected: beta-chair"
  - assert_text:
      contains: "beta-chair"
  - resize:
      width: 80
      height: 24
  - press: ESC
  - wait_for_text:
      text: "TUI-LAB-SAMPLE"
  - press: "/"
  - type: "table"
  - wait_for_text:
      text: "Search: table"
  - assert_region:
      x: 0
      y: 1
      width: 80
      height: 5
      contains: "alpha-table"
  - snapshot:
      name: "proof"
cleanup:
  - press: ESC
assertions:
  - exit_code: 0
"#
    )
}

/// Feed one PTY chunk into the grid and return the fresh screen text.
fn pump(sess: &PtySession, emu: &mut Emulator) -> String {
    let chunk = sess.poll(Duration::from_millis(100));
    emu.feed(&chunk);
    emu.text()
}

async fn live_screen(sess: &PtySession, emu: &mut Emulator, needle: &str, timeout: Duration) {
    let outcome = wait_for_text(|| pump(sess, emu), needle, false, timeout, POLL).await;
    assert!(
        outcome.found,
        "wait_for_text {needle:?} timed out, last screen:\n{}",
        outcome.last_screen
    );
}

/// Char-safe region slice of screen text.
fn region_text(screen: &str, x: u16, y: u16, width: u16, height: u16) -> String {
    screen
        .lines()
        .skip(y as usize)
        .take(height as usize)
        .map(|line| {
            line.chars()
                .skip(x as usize)
                .take(width as usize)
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn eval_text_assertion(sess: &PtySession, emu: &mut Emulator, assertion: &TextAssertion) {
    let screen = pump(sess, emu);
    let view = ScreenView::live(&screen, emu.cursor(), true);
    if let Some(needle) = &assertion.contains {
        let verdict = evaluate(&Condition::TextVisible(needle.clone()), &view);
        assert!(verdict.passed, "{}", verdict.detail);
    }
    if let Some(needle) = &assertion.not_contains {
        let verdict = evaluate(&Condition::TextNotVisible(needle.clone()), &view);
        assert!(verdict.passed, "{}", verdict.detail);
    }
    if let Some(pattern) = &assertion.regex {
        let verdict = evaluate(&Condition::TextRegex(pattern.clone()), &view);
        assert!(verdict.passed, "{}", verdict.detail);
    }
}

fn take_snapshot(sess: &PtySession, emu: &mut Emulator, name: &str) {
    let screen = pump(sess, emu);
    let dir = std::env::temp_dir().join("tuilab-proof-snap");
    let path = save_text(&dir, name, 80, 24, &screen).expect("save snapshot");
    let back = std::fs::read_to_string(&path).expect("reload snapshot");
    let outcome = compare_text(&back, &screen, &[]);
    assert!(outcome.equal, "snapshot self-compare:\n{}", outcome.diff);
    let _ = std::fs::remove_dir_all(&dir);
}

async fn run_steps(sess: &mut PtySession, emu: &mut Emulator, steps: &[Step]) {
    for step in steps {
        match step {
            Step::Press(key) => {
                let bytes = encode_key(key).expect("encode key");
                sess.write_all(&bytes).expect("write key");
            }
            Step::Type(text) => {
                sess.write_all(&encode_text(text)).expect("write text");
            }
            Step::WaitForText(wait) => {
                live_screen(sess, emu, &wait.text, wait.timeout.unwrap_or(WAIT_TIMEOUT)).await;
            }
            Step::Sleep(sleep) => {
                std::thread::sleep(sleep.0);
            }
            Step::Resize(to) => {
                sess.resize(to.width, to.height).expect("resize pty");
                emu.resize(to.width as usize, to.height as usize);
            }
            Step::AssertText(assertion) | Step::Expect(assertion) => {
                eval_text_assertion(sess, emu, assertion);
            }
            Step::AssertRegion(region) => {
                let screen = pump(sess, emu);
                let area = region_text(&screen, region.x, region.y, region.width, region.height);
                assert!(
                    area.contains(&region.contains),
                    "region missing {:?}, got:\n{area}",
                    region.contains
                );
            }
            Step::Snapshot(take) | Step::Screenshot(take) => {
                take_snapshot(sess, emu, &take.name);
            }
            Step::WaitForExit(wait) => {
                let start = std::time::Instant::now();
                loop {
                    if sess.try_wait().expect("poll exit").is_some() {
                        break;
                    }
                    assert!(
                        start.elapsed() < wait.timeout,
                        "process did not exit in {:?}",
                        wait.timeout
                    );
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn yaml_suite_runs_end_to_end() {
    let bin = fixture_bin();
    // Forward slashes: valid on Windows and safe inside YAML double quotes.
    let bin = bin.to_string_lossy().replace('\\', "/");
    let file = TestFile::from_yaml(&suite_yaml(&bin)).expect("parse proof suite");

    let opts = SpawnOptions {
        command: file.application.command.clone(),
        cols: file.terminal.width,
        rows: file.terminal.height,
        ..SpawnOptions::default()
    };
    let mut sess = PtySession::spawn(&opts).expect("spawn fixture");
    let mut emu = Emulator::new(
        file.terminal.width as usize,
        file.terminal.height as usize,
        sess.writer_sink(),
    );

    run_steps(&mut sess, &mut emu, &file.setup).await;
    run_steps(&mut sess, &mut emu, &file.steps).await;
    run_steps(&mut sess, &mut emu, &file.cleanup).await;

    let status = sess.close(Some(b"q"), None).expect("bounded close");
    for assertion in &file.assertions {
        match assertion {
            SuiteAssertion::ExitCode(0) => {
                assert!(status.success(), "expected exit 0, got {status:?}");
            }
            SuiteAssertion::ExitCode(expected) => {
                assert!(
                    !status.success(),
                    "expected nonzero exit {expected}, got success"
                );
            }
            // Conservative v1 semantics: any nonzero exit counts as a crash.
            SuiteAssertion::ProcessNotCrashed(expected) => {
                let crashed = !status.success();
                assert_eq!(
                    !crashed, *expected,
                    "crash expectation mismatch (crashed={crashed})"
                );
            }
        }
    }
}
