//! Runner outcomes: green suite, abort on failure, launch errors, kill path.
//!
//! The passing path is covered end to end by `single_session.rs`; here the
//! failure and lifecycle edges are pinned down.

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use tui_lab_core::{run_file, RunOptions, StepHook};
use tui_lab_protocol::TestFile;

/// Build the fixture binary on demand; return its path.
fn fixture_bin() -> String {
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
    dir.join("target")
        .join("debug")
        .join(bin)
        .to_string_lossy()
        .replace('\\', "/")
}

fn opts() -> RunOptions {
    RunOptions {
        wait_default: Duration::from_secs(8),
        ..RunOptions::default()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn failing_assert_aborts_with_step_index() {
    let file = TestFile::from_yaml(&format!(
        r#"
schema: tui-lab/v1
name: failing
application:
  command: "{}"
steps:
  - wait_for_text:
      text: "TUI-LAB-SAMPLE"
  - assert_text:
      contains: "no-such-screen"
  - press: q
assertions:
  - exit_code: 0
"#,
        fixture_bin()
    ))
    .expect("parse");
    let result = run_file(&file, &opts()).await.expect("run completes");
    assert!(!result.passed);
    let failure = result.failure.expect("failure recorded");
    // The assert is the second executed step (index 1).
    assert_eq!(failure.step_index, 1);
    assert!(failure.step.contains("assert_text"));
    assert!(!failure.last_screen.is_empty());
    assert!(!failure.input_history.is_empty());
    // Steps after the failure never ran: no press recorded past the assert.
    assert!(
        result.steps.len() < 3,
        "aborted early, got {} steps",
        result.steps.len()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn empty_command_fails_before_launch() {
    let file = TestFile::from_yaml(
        r#"
schema: tui-lab/v1
name: bad-launch
application:
  command: ""
steps:
  - press: q
"#,
    )
    .expect("parse");
    let err = run_file(&file, &opts()).await.expect_err("launch fails");
    assert!(err.to_string().contains("launch failed"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn quitless_suite_is_reaped_by_kill() {
    let file = TestFile::from_yaml(&format!(
        r#"
schema: tui-lab/v1
name: no-quit
application:
  command: "{}"
steps:
  - wait_for_text:
      text: "TUI-LAB-SAMPLE"
"#,
        fixture_bin()
    ))
    .expect("parse");
    let started = std::time::Instant::now();
    // No quit input anywhere: close() must kill within grace, never hang.
    let result = run_file(&file, &opts()).await.expect("run completes");
    assert!(
        started.elapsed() < Duration::from_secs(30),
        "kill path must be bounded"
    );
    // Killed process: exit_success is false, suite did not "pass" only if an
    // exit assertion says so — with none, steps passing is enough.
    assert!(result.steps.iter().all(|step| step.passed));
    assert_eq!(result.exit_success, Some(false));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn step_hook_aborts_early_but_cleanup_runs() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let file = TestFile::from_yaml(&format!(
        r#"schema: tui-lab/v1
name: hook-abort
application:
  command: "{}"
steps:
  - wait_for_text:
      text: "TUI-LAB-SAMPLE"
  - press: DOWN
  - press: ENTER
  - press: q
cleanup:
  - press: q
"#,
        fixture_bin()
    ))
    .expect("parse");
    let calls = Arc::new(AtomicUsize::new(0));
    let probe = Arc::clone(&calls);
    let hook: StepHook = Arc::new(move |_| {
        // Abort from the second call on: two main steps land, then cleanup
        // runs its single step and stops as well.
        probe.fetch_add(1, Ordering::SeqCst) < 1
    });
    let opts = RunOptions {
        step_hook: Some(hook),
        ..RunOptions::default()
    };
    let result = run_file(&file, &opts).await.expect("run completes");
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert_eq!(result.steps.len(), 3);
    assert!(result.steps[2].kind.starts_with("cleanup:"));
}
