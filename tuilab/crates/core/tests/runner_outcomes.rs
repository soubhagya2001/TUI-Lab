//! Runner outcomes: green suite, abort on failure, launch errors, kill path.
//!
//! The passing path is covered end to end by `single_session.rs`; here the
//! failure and lifecycle edges are pinned down.

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use tui_lab_core::{run_file, run_file_bounded, RunOptions, StepHook};
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn assert_with_timeout_polls_then_fails() {
    // C2: an assert with `timeout` keeps polling instead of failing on the
    // first screen — the step takes ~the timeout, then reports the verdict.
    let file = TestFile::from_yaml(&format!(
        r#"
schema: tui-lab/v1
name: assert-timeout
application:
  command: "{}"
steps:
  - wait_for_text:
      text: "TUI-LAB-SAMPLE"
  - assert_text:
      contains: "no-such-screen"
      timeout: 1s
  - press: q
"#,
        fixture_bin()
    ))
    .expect("parse");
    let started = std::time::Instant::now();
    let result = run_file(&file, &opts()).await.expect("run completes");
    assert!(!result.passed, "absent text must fail");
    assert!(
        started.elapsed() >= Duration::from_millis(900),
        "assert must poll for ~the timeout, took {:?}",
        started.elapsed()
    );
    let failure = result.failure.expect("failure recorded");
    assert!(failure.step.contains("assert_text"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn assert_reaches_full_taxonomy() {
    // C3: exact/cursor/exit/health/change checks are reachable from YAML.
    let file = TestFile::from_yaml(&format!(
        r#"
schema: tui-lab/v1
name: taxonomy
application:
  command: "{}"
steps:
  - wait_for_text:
      text: "TUI-LAB-SAMPLE"
  - press: DOWN
  - assert_text:
      screen_changed: true
      not_crashed: true
  - press: ENTER
  - assert_text:
      contains: "beta-chair"
  - press: q
  - assert_text:
      exit_code: 0
      timeout: 5s
"#,
        fixture_bin()
    ))
    .expect("parse");
    let result = run_file(&file, &opts()).await.expect("run completes");
    assert!(
        result.passed,
        "taxonomy suite must pass: {:?}",
        result.failure
    );
    assert!(result.steps.iter().all(|step| step.passed));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn snapshot_uses_size_scoped_store() {
    // C4: goldens live at <suite>/<name>/<WxH>.txt — sizes must not collide
    // in a flat <name>.txt.
    let dir = std::env::temp_dir().join(format!("tuilab-snap-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let file = TestFile::from_yaml(&format!(
        r#"
schema: tui-lab/v1
name: scoped
application:
  command: "{}"
terminal:
  width: 120
  height: 40
steps:
  - wait_for_text:
      text: "TUI-LAB-SAMPLE"
  - snapshot:
      name: boot
  - press: q
"#,
        fixture_bin()
    ))
    .expect("parse");
    let opts = RunOptions {
        snapshot_dir: dir.clone(),
        ..RunOptions::default()
    };
    let first = run_file(&file, &opts).await.expect("run completes");
    assert!(!first.passed, "first run writes .new and fails");
    let scoped_new = dir.join("scoped").join("boot").join("120x40.new");
    assert!(scoped_new.is_file(), "size-scoped .new written");
    assert!(
        !dir.join("scoped").join("boot.txt").exists(),
        "no flat legacy golden"
    );
    // Approve and re-run: now it compares and passes.
    let golden = scoped_new.with_extension("txt");
    std::fs::rename(&scoped_new, &golden).expect("approve golden");
    let second = run_file(&file, &opts).await.expect("run completes");
    assert!(second.passed, "approved golden must match");
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn suite_terminal_timeout_bounds_run() {
    // C5: `terminal.timeout` caps the whole suite via run_file_bounded.
    let file = TestFile::from_yaml(&format!(
        r#"
schema: tui-lab/v1
name: bounded
application:
  command: "{}"
terminal:
  width: 120
  height: 40
  timeout: 1s
steps:
  - wait_for_text:
      text: "text-that-never-appears"
      timeout: 5s
"#,
        fixture_bin()
    ))
    .expect("parse");
    let err = run_file_bounded(&file, &opts())
        .await
        .expect_err("suite bound must fire first");
    assert!(
        err.to_string().starts_with("timed out"),
        "exit-4 mapping, got: {err}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn zero_resize_is_rejected() {
    // R6: zero dimensions are invalid input, never a silent PTY call.
    let file = TestFile::from_yaml(&format!(
        r#"
schema: tui-lab/v1
name: zero-resize
application:
  command: "{}"
steps:
  - wait_for_text:
      text: "TUI-LAB-SAMPLE"
  - resize:
      width: 0
      height: 24
  - press: q
"#,
        fixture_bin()
    ))
    .expect("parse");
    let err = run_file(&file, &opts())
        .await
        .expect_err("zero resize must fail");
    assert!(err.to_string().contains("nonzero"), "got: {err}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn oversize_resize_is_clamped() {
    // R6: absurd dimensions clamp to materialization limits and say so.
    let file = TestFile::from_yaml(&format!(
        r#"
schema: tui-lab/v1
name: big-resize
application:
  command: "{}"
steps:
  - wait_for_text:
      text: "TUI-LAB-SAMPLE"
  - resize:
      width: 9999
      height: 9999
  - press: q
assertions:
  - exit_code: 0
"#,
        fixture_bin()
    ))
    .expect("parse");
    let result = run_file(&file, &opts()).await.expect("run completes");
    assert!(result.passed, "clamped resize must pass");
    assert!(
        result.steps[1].detail.contains("clamped"),
        "detail names the clamp: {}",
        result.steps[1].detail
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn oob_region_fails_with_named_bounds() {
    // R3: an out-of-bounds region is a step failure naming the grid — not
    // an infra abort, not a misleading empty mismatch.
    let file = TestFile::from_yaml(&format!(
        r#"
schema: tui-lab/v1
name: oob-region
application:
  command: "{}"
steps:
  - wait_for_text:
      text: "TUI-LAB-SAMPLE"
  - assert_region:
      x: 500
      y: 0
      width: 10
      height: 1
      contains: "x"
  - press: q
"#,
        fixture_bin()
    ))
    .expect("parse");
    let result = run_file(&file, &opts()).await.expect("run completes");
    assert!(!result.passed);
    let failure = result.failure.expect("failure recorded");
    assert!(
        failure.expected.contains("outside 120x40"),
        "{}",
        failure.expected
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn step_budget_fails_slow_steps() {
    // D2: a 1ms step budget fails even passing steps, with evidence.
    let file = TestFile::from_yaml(&format!(
        r#"
schema: tui-lab/v1
name: budgeted-step
application:
  command: "{}"
budgets:
  step: 1ms
steps:
  - wait_for_text:
      text: "TUI-LAB-SAMPLE"
  - press: q
"#,
        fixture_bin()
    ))
    .expect("parse");
    let result = run_file(&file, &opts()).await.expect("run completes");
    assert!(!result.passed, "over-budget step must fail");
    let failure = result.failure.expect("failure recorded");
    assert!(
        failure.expected.contains("budget exceeded"),
        "{}",
        failure.expected
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn suite_budget_fails_long_runs() {
    // D2: a 1ms suite budget fails the run as a test result (not infra).
    let file = TestFile::from_yaml(&format!(
        r#"
schema: tui-lab/v1
name: budgeted-suite
application:
  command: "{}"
budgets:
  suite: 1ms
steps:
  - wait_for_text:
      text: "TUI-LAB-SAMPLE"
  - press: q
"#,
        fixture_bin()
    ))
    .expect("parse");
    let result = run_file(&file, &opts()).await.expect("run completes");
    assert!(!result.passed);
    let failure = result.failure.expect("failure recorded");
    assert_eq!(failure.step, "budget");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn startup_budget_fails_blank_boots() {
    // D2: a command that never paints fails the startup budget (no steps run).
    #[cfg(windows)]
    let (command, args) = (
        "powershell".to_string(),
        vec![
            "-NoProfile".to_string(),
            "-Command".to_string(),
            "Start-Sleep 5".to_string(),
        ],
    );
    #[cfg(not(windows))]
    let (command, args) = ("sleep".to_string(), vec!["5".to_string()]);
    // Split command + args portably.
    let args_yaml = args
        .iter()
        .map(|arg| format!("    - \"{arg}\"\n"))
        .collect::<String>();
    let file = TestFile::from_yaml(&format!(
        "schema: tui-lab/v1\nname: budgeted-startup\napplication:\n  command: \"{command}\"\n  args:\n{args_yaml}budgets:\n  startup: 500ms\nsteps:\n  - wait_for_text:\n      text: \"TUI-LAB-SAMPLE\"\n",
    ))
    .expect("parse");
    let started = std::time::Instant::now();
    let result = run_file(&file, &opts()).await.expect("run completes");
    assert!(!result.passed);
    assert!(result.steps.is_empty(), "no steps ran");
    let failure = result.failure.expect("failure recorded");
    assert_eq!(failure.step, "startup");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(10),
        "budget bounds the wait, took {:?}",
        started.elapsed()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn role_assertion_finds_fixture_button() {
    // E1: YAML `role`/`name` walks the heuristic a11y tree end to end.
    // The fixture list footer renders `[q] quit` — a detectable button.
    let file = TestFile::from_yaml(&format!(
        r#"
schema: tui-lab/v1
name: role-pass
application:
  command: "{}"
steps:
  - wait_for_text:
      text: "TUI-LAB-SAMPLE"
  - assert_text:
      role: button
      name: q
  - press: q
"#,
        fixture_bin()
    ))
    .expect("parse");
    let result = run_file(&file, &opts()).await.expect("run completes");
    assert!(
        result.passed,
        "button role must be found: {:?}",
        result.failure
    );
    assert!(result.steps.iter().all(|step| step.passed));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn role_assertion_fails_cleanly_when_absent() {
    // E1: a missing widget is a test failure naming the role — not an
    // infra abort, not a silent pass.
    let file = TestFile::from_yaml(&format!(
        r#"
schema: tui-lab/v1
name: role-fail
application:
  command: "{}"
steps:
  - wait_for_text:
      text: "TUI-LAB-SAMPLE"
  - assert_text:
      role: button
      name: cancel
  - press: q
"#,
        fixture_bin()
    ))
    .expect("parse");
    let result = run_file(&file, &opts()).await.expect("run completes");
    assert!(!result.passed, "absent button must fail");
    let failure = result.failure.expect("failure recorded");
    assert!(failure.step.contains("assert_text"));
    assert!(
        failure.expected.contains("button") || failure.expected.contains("cancel"),
        "failure names the missing widget: {}",
        failure.expected
    );
}
