//! MCP tools: router inventory, security rejections, Mode A loop, Mode B run.
//!
//! Tool methods are driven directly (same code the stdio server calls);
//! the stdio handshake itself is covered by the CI probe step.

use std::path::PathBuf;
use std::process::Command;

use rmcp::handler::server::wrapper::Parameters;
use tui_lab_mcp::handler::TuiLabHandler;
use tui_lab_mcp::security::{Allowlist, AuditConfig, AuditField};
use tui_lab_mcp::tools::{
    AssertParams, CloseParams, LaunchParams, PressParams, ResizeParams, RunTestParams,
    ScreenParams, SnapshotParams, TypeParams, WaitParams,
};

fn scratch_root(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tuilab-mcp-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch root");
    dir
}

fn open_handler(name: &str) -> (TuiLabHandler, PathBuf) {
    let root = scratch_root(name);
    let allow = Allowlist::from_sources(&[".*"]).expect("permissive test allowlist");
    (
        TuiLabHandler::new(root.clone(), allow, AuditConfig::disabled()),
        root,
    )
}

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

#[test]
fn router_lists_exactly_ten_tools() {
    let (handler, _root) = open_handler("inventory");
    let mut names: Vec<String> = handler
        .router()
        .list_all()
        .iter()
        .map(|tool| tool.name.to_string())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "tui_assert",
            "tui_close",
            "tui_launch",
            "tui_press",
            "tui_resize",
            "tui_run_test",
            "tui_screen",
            "tui_snapshot",
            "tui_type",
            "tui_wait_for_text",
        ]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn forbidden_commands_are_rejected() {
    let root = scratch_root("forbidden");
    let allow = Allowlist::defaults();
    let handler = TuiLabHandler::new(root, allow, AuditConfig::disabled());
    let result = handler
        .tui_launch(Parameters(LaunchParams {
            command: "rm".to_string(),
            args: vec!["-rf".to_string(), "/".to_string()],
            cwd: None,
            width: 120,
            height: 40,
            env: Default::default(),
        }))
        .await;
    let err = match result {
        Ok(_) => panic!("rm must be forbidden"),
        Err(err) => err,
    };
    assert!(err.contains("FORBIDDEN_COMMAND"), "{err}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cwd_escape_is_rejected() {
    let (handler, _root) = open_handler("jail");
    let result = handler
        .tui_launch(Parameters(LaunchParams {
            command: "./myapp".to_string(),
            args: vec![],
            cwd: Some("../..".to_string()),
            width: 120,
            height: 40,
            env: Default::default(),
        }))
        .await;
    let err = match result {
        Ok(_) => panic!("escape must be rejected"),
        Err(err) => err,
    };
    assert!(err.contains("escapes project root"), "{err}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mode_a_loop_against_fixture() {
    let (handler, _root) = open_handler("mode-a");
    let launch = handler
        .tui_launch(Parameters(LaunchParams {
            command: fixture_bin(),
            args: vec![],
            cwd: None,
            width: 120,
            height: 40,
            env: Default::default(),
        }))
        .await
        .expect("launch")
        .0;
    assert_eq!(launch.status, "running");
    let id = launch.session_id;

    let wait = handler
        .tui_wait_for_text(Parameters(WaitParams {
            session_id: id.clone(),
            text: "TUI-LAB-SAMPLE".to_string(),
            regex: false,
            timeout_ms: 10_000,
        }))
        .await
        .expect("wait")
        .0;
    assert!(wait.found, "boot screen");

    let press = handler
        .tui_press(Parameters(PressParams {
            session_id: id.clone(),
            key: "DOWN".to_string(),
        }))
        .await
        .expect("press")
        .0;
    assert!(press.ok);

    handler
        .tui_press(Parameters(PressParams {
            session_id: id.clone(),
            key: "ENTER".to_string(),
        }))
        .await
        .expect("enter");

    let assert_out = handler
        .tui_assert(Parameters(AssertParams {
            session_id: id.clone(),
            assertion: serde_json::json!({"type": "text_visible", "text": "beta-chair"}),
        }))
        .await
        .expect("assert")
        .0;
    assert!(assert_out.passed, "{}", assert_out.detail);

    let screen = handler
        .tui_screen(Parameters(ScreenParams {
            session_id: id.clone(),
            styled: true,
        }))
        .await
        .expect("styled screen")
        .0;
    let cells = screen.cells.expect("cells present when styled");
    assert!(!cells.is_empty());
    let title: String = cells
        .iter()
        .filter(|cell| cell.y == 0)
        .map(|cell| cell.char.as_str())
        .collect();
    assert!(title.contains("TUI-LAB-SAMPLE"), "{title}");

    let screen = handler
        .tui_screen(Parameters(ScreenParams {
            session_id: id.clone(),
            styled: false,
        }))
        .await
        .expect("screen")
        .0;
    assert!(screen.cells.is_none(), "cells omitted unless styled");
    assert!(screen.text.contains("Selected: beta-chair"));
    assert_eq!((screen.width, screen.height), (120, 40));

    // Snapshot writes the golden first, matches on second call.
    let first = handler
        .tui_snapshot(Parameters(SnapshotParams {
            session_id: id.clone(),
            name: "mode-a".to_string(),
        }))
        .await
        .expect("snapshot")
        .0;
    assert!(first.saved);
    let second = handler
        .tui_snapshot(Parameters(SnapshotParams {
            session_id: id.clone(),
            name: "mode-a".to_string(),
        }))
        .await
        .expect("snapshot")
        .0;
    assert!(!second.saved);
    assert!(second.diff.is_none());

    // Unknown keys and sessions are tool errors, not panics.
    assert!(handler
        .tui_press(Parameters(PressParams {
            session_id: id.clone(),
            key: "F13".to_string(),
        }))
        .await
        .is_err());
    assert!(handler
        .tui_screen(Parameters(ScreenParams {
            session_id: "sess_404".to_string(),
            styled: false,
        }))
        .await
        .is_err());

    // Sensitive typing never echoes.
    let typed = handler
        .tui_type(Parameters(TypeParams {
            session_id: id.clone(),
            text: "s3cret".to_string(),
            sensitive: true,
        }))
        .await
        .expect("type")
        .0;
    assert!(typed.ok);

    // Quit from the detail screen, then close must reap a clean exit.
    // Quit rides inside close (one call, no press/close race).
    let close = handler
        .tui_close(Parameters(CloseParams {
            session_id: id.clone(),
            quit: Some("q".to_string()),
        }))
        .await
        .expect("close")
        .0;
    assert!(close.success, "clean quit reaps exit success");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_waits_do_not_serialize() {
    // S1: two slow waits on different sessions must overlap. Under the old
    // global-lock-across-await design they ran back-to-back (~4s+ for two
    // 2s waits); per-tick locking finishes in ~one timeout.
    use std::time::{Duration, Instant};

    let (handler, _root) = open_handler("concurrent-waits");
    let bin = fixture_bin();
    let mut ids = Vec::new();
    for _ in 0..2 {
        let launch = handler
            .tui_launch(Parameters(LaunchParams {
                command: bin.clone(),
                args: vec![],
                cwd: None,
                width: 120,
                height: 40,
                env: Default::default(),
            }))
            .await
            .expect("launch")
            .0;
        ids.push(launch.session_id);
    }
    let unmatchable = "text-that-never-appears-zzz".to_string();
    let start = Instant::now();
    let (first, second) = tokio::join!(
        handler.tui_wait_for_text(Parameters(WaitParams {
            session_id: ids[0].clone(),
            text: unmatchable.clone(),
            regex: false,
            timeout_ms: 2000,
        })),
        handler.tui_wait_for_text(Parameters(WaitParams {
            session_id: ids[1].clone(),
            text: unmatchable,
            regex: false,
            timeout_ms: 2000,
        })),
    );
    let elapsed = start.elapsed();
    assert!(!first.expect("wait one").0.found);
    assert!(!second.expect("wait two").0.found);
    assert!(
        elapsed < Duration::from_millis(3500),
        "waits serialized behind one lock: {elapsed:?}"
    );
    for id in &ids {
        handler
            .tui_close(Parameters(CloseParams {
                session_id: id.clone(),
                quit: Some("q".to_string()),
            }))
            .await
            .expect("close");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn run_test_rejects_path_traversal() {
    // S3: absolute paths outside the root and `..` escapes never read.
    let (handler, root) = open_handler("traversal");
    let outside =
        std::env::temp_dir().join(format!("tuilab-traversal-{}.yaml", std::process::id()));
    std::fs::write(&outside, "schema: tui-lab/v1").expect("outside suite");
    let err = match handler
        .tui_run_test(Parameters(RunTestParams {
            test_file: outside.to_string_lossy().into_owned(),
            terminal: None,
        }))
        .await
    {
        Ok(_) => panic!("absolute outside must fail"),
        Err(err) => err,
    };
    assert!(err.contains("escapes project root"), "{err}");
    let err = match handler
        .tui_run_test(Parameters(RunTestParams {
            test_file: "../traversal.yaml".to_string(),
            terminal: None,
        }))
        .await
    {
        Ok(_) => panic!("dotdot must fail"),
        Err(err) => err,
    };
    assert!(
        err.contains("escapes project root") || err.contains("suite file"),
        "{err}"
    );
    // `..` to a REAL file outside still hits the jail check, not just
    // missing-file handling.
    let dotdot = format!("../tuilab-traversal-{}.yaml", std::process::id());
    let err = match handler
        .tui_run_test(Parameters(RunTestParams {
            test_file: dotdot,
            terminal: None,
        }))
        .await
    {
        Ok(_) => panic!("dotdot to real file must fail"),
        Err(err) => err,
    };
    assert!(err.contains("escapes project root"), "{err}");
    let _ = std::fs::remove_file(&outside);
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn nonzero_exit_code_asserts() {
    // C1: ExitCode(2) must pass with the numeric code — nonzero exits are
    // codes, not crashes. Under the old mapping this looped forever on
    // "process still running".
    use std::time::{Duration, Instant};

    let (handler, _root) = open_handler("exit-code");
    #[cfg(windows)]
    let (command, args) = (
        "cmd".to_string(),
        vec!["/C".to_string(), "exit 2".to_string()],
    );
    #[cfg(not(windows))]
    let (command, args) = (
        "sh".to_string(),
        vec!["-c".to_string(), "exit 2".to_string()],
    );
    let id = handler
        .tui_launch(Parameters(LaunchParams {
            command,
            args,
            cwd: None,
            width: 80,
            height: 24,
            env: Default::default(),
        }))
        .await
        .expect("launch")
        .0
        .session_id;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let out = handler
            .tui_assert(Parameters(AssertParams {
                session_id: id.clone(),
                assertion: serde_json::json!({"type": "exit_code", "code": 2}),
            }))
            .await
            .expect("assert")
            .0;
        if out.passed {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "exit code 2 never observed: {}",
            out.detail
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    handler
        .tui_close(Parameters(CloseParams {
            session_id: id,
            quit: None,
        }))
        .await
        .expect("close");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn zero_wait_timeout_falls_back_to_default() {
    // R6: explicit timeout_ms 0 means "use the default", not "fail now".
    let (handler, _root) = open_handler("zero-timeout");
    let id = handler
        .tui_launch(Parameters(LaunchParams {
            command: fixture_bin(),
            args: vec![],
            cwd: None,
            width: 120,
            height: 40,
            env: Default::default(),
        }))
        .await
        .expect("launch")
        .0
        .session_id;
    let wait = handler
        .tui_wait_for_text(Parameters(WaitParams {
            session_id: id.clone(),
            text: "TUI-LAB-SAMPLE".to_string(),
            regex: false,
            timeout_ms: 0,
        }))
        .await
        .expect("wait")
        .0;
    assert!(wait.found, "zero timeout falls back to default");
    handler
        .tui_close(Parameters(CloseParams {
            session_id: id,
            quit: Some("q".to_string()),
        }))
        .await
        .expect("close");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn idle_sessions_reaped_on_tool_entry() {
    // R10: the reaper runs on every tool entry, not just launches — an
    // untouched session past the idle timeout is gone at the next call.
    let root = scratch_root("reap");
    let allow = Allowlist::from_sources(&[".*"]).expect("permissive");
    let handler = TuiLabHandler::with_idle(
        root,
        allow,
        AuditConfig::disabled(),
        std::time::Duration::from_millis(100),
    );
    let id = handler
        .tui_launch(Parameters(LaunchParams {
            command: fixture_bin(),
            args: vec![],
            cwd: None,
            width: 120,
            height: 40,
            env: Default::default(),
        }))
        .await
        .expect("launch")
        .0
        .session_id;
    // Fresh session serves fine.
    handler
        .tui_screen(Parameters(ScreenParams {
            session_id: id.clone(),
            styled: false,
        }))
        .await
        .expect("fresh session serves");
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    // Past idle: the next tool entry reaps first, so the read fails.
    let err = match handler
        .tui_screen(Parameters(ScreenParams {
            session_id: id,
            styled: false,
        }))
        .await
    {
        Ok(_) => panic!("idle session must be reaped"),
        Err(err) => err,
    };
    assert!(err.contains("unknown session"), "{err}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resize_reports_actual_dims_and_validates() {
    let (handler, _root) = open_handler("resize");
    let id = handler
        .tui_launch(Parameters(LaunchParams {
            command: fixture_bin(),
            args: vec![],
            cwd: None,
            width: 120,
            height: 40,
            env: Default::default(),
        }))
        .await
        .expect("launch")
        .0
        .session_id;
    // Normal resize reports back the requested dims; the grid follows
    // (visible through tui_screen).
    let out = handler
        .tui_resize(Parameters(ResizeParams {
            session_id: id.clone(),
            width: 80,
            height: 24,
        }))
        .await
        .expect("resize")
        .0;
    assert!(out.ok);
    assert_eq!((out.width, out.height), (80, 24));
    let screen = handler
        .tui_screen(Parameters(ScreenParams {
            session_id: id.clone(),
            styled: false,
        }))
        .await
        .expect("screen")
        .0;
    assert_eq!((screen.width, screen.height), (80, 24));
    // Oversize clamps to materialization limits and says so.
    let clamped = handler
        .tui_resize(Parameters(ResizeParams {
            session_id: id.clone(),
            width: 9999,
            height: 9999,
        }))
        .await
        .expect("clamped resize")
        .0;
    assert!(clamped.ok);
    assert_eq!((clamped.width, clamped.height), (500, 200));
    // Zero dims are rejected, not silently applied.
    assert!(handler
        .tui_resize(Parameters(ResizeParams {
            session_id: id.clone(),
            width: 0,
            height: 24,
        }))
        .await
        .is_err());
    // Unknown sessions error.
    assert!(handler
        .tui_resize(Parameters(ResizeParams {
            session_id: "sess_404".to_string(),
            width: 80,
            height: 24,
        }))
        .await
        .is_err());
    handler
        .tui_close(Parameters(CloseParams {
            session_id: id,
            quit: Some("q".to_string()),
        }))
        .await
        .expect("close");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn audit_log_records_calls_and_redacts_secrets() {
    // Opt-in JSONL audit: successes + failures logged, sensitive text redacted.
    let root = scratch_root("audit");
    let allow = Allowlist::from_sources(&[".*"]).expect("permissive");
    let audit_path = root.join("audit.jsonl");
    let audit = AuditConfig {
        enabled: true,
        path: audit_path.clone(),
        fields: vec![
            AuditField::Timestamp,
            AuditField::Tool,
            AuditField::Session,
            AuditField::Result,
            AuditField::Args,
            AuditField::Elapsed,
        ],
    };
    let handler = TuiLabHandler::new(root.clone(), allow, audit);
    let id = handler
        .tui_launch(Parameters(LaunchParams {
            command: fixture_bin(),
            args: vec![],
            cwd: None,
            width: 120,
            height: 40,
            env: Default::default(),
        }))
        .await
        .expect("launch")
        .0
        .session_id;
    handler
        .tui_type(Parameters(TypeParams {
            session_id: id.clone(),
            text: "s3cret".to_string(),
            sensitive: true,
        }))
        .await
        .expect("type");
    assert!(handler
        .tui_press(Parameters(PressParams {
            session_id: id.clone(),
            key: "F13".to_string(),
        }))
        .await
        .is_err());
    handler
        .tui_close(Parameters(CloseParams {
            session_id: id.clone(),
            quit: Some("q".to_string()),
        }))
        .await
        .expect("close");
    let raw = std::fs::read_to_string(&audit_path).expect("audit file");
    assert!(!raw.contains("s3cret"), "secrets never reach the audit log");
    let lines: Vec<serde_json::Value> = raw
        .lines()
        .map(|line| serde_json::from_str(line).expect("JSONL line"))
        .collect();
    assert_eq!(lines.len(), 4, "launch + type + press + close");
    for line in &lines {
        for field in [
            "timestamp",
            "tool",
            "session_id",
            "result",
            "args",
            "elapsed_ms",
        ] {
            assert!(line.get(field).is_some(), "field {field} in {line}");
        }
    }
    let tools: Vec<&str> = lines
        .iter()
        .map(|line| line.get("tool").and_then(|t| t.as_str()).unwrap_or("?"))
        .collect();
    assert_eq!(tools, ["tui_launch", "tui_type", "tui_press", "tui_close"]);
    let typed = &lines[1];
    assert_eq!(
        typed
            .get("args")
            .and_then(|a| a.get("text"))
            .and_then(|t| t.as_str()),
        Some("[redacted]")
    );
    assert_eq!(typed.get("result").and_then(|r| r.as_str()), Some("ok"));
    let failed = &lines[2];
    assert_ne!(
        failed.get("result").and_then(|r| r.as_str()),
        Some("ok"),
        "failures log the error"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mode_b_runs_yaml_suite() {
    let (handler, root) = open_handler("mode-b");
    // Mode B resolves suites under the handler root; the test writes its own
    // suite there with an absolute fixture command (the repo smoke.yaml uses
    // a cargo-relative command valid only under tuilab/). S3 jails reads to
    // the root, so the suite must live inside it.
    let suite = root.join("mini.yaml");
    std::fs::write(
        &suite,
        format!(
            r#"schema: tui-lab/v1
name: mini
application:
  command: "{}"
steps:
  - wait_for_text:
      text: "TUI-LAB-SAMPLE"
  - press: q
assertions:
  - exit_code: 0
"#,
            fixture_bin()
        ),
    )
    .expect("write suite");
    let out = handler
        .tui_run_test(Parameters(RunTestParams {
            test_file: suite.to_string_lossy().into_owned(),
            terminal: None,
        }))
        .await
        .expect("run_test")
        .0;
    assert_eq!(out.status, "passed", "{:?}", out.failures);
    assert!(out.failures.is_empty());
    let _ = std::fs::remove_dir_all(&root);
}
