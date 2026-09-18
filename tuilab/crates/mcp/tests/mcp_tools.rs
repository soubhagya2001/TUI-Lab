//! MCP tools: router inventory, security rejections, Mode A loop, Mode B run.
//!
//! Tool methods are driven directly (same code the stdio server calls);
//! the stdio handshake itself is covered by the CI probe step.

use std::path::PathBuf;
use std::process::Command;

use rmcp::handler::server::wrapper::Parameters;
use tui_lab_mcp::handler::TuiLabHandler;
use tui_lab_mcp::security::Allowlist;
use tui_lab_mcp::tools::{
    AssertParams, CloseParams, LaunchParams, PressParams, RunTestParams, ScreenParams,
    SnapshotParams, TypeParams, WaitParams,
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
    (TuiLabHandler::new(root.clone(), allow), root)
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
fn router_lists_exactly_nine_tools() {
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
    let handler = TuiLabHandler::new(root, allow);
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
            styled: false,
        }))
        .await
        .expect("screen")
        .0;
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
    handler
        .tui_press(Parameters(PressParams {
            session_id: id.clone(),
            key: "q".to_string(),
        }))
        .await
        .expect("quit");

    let close = handler
        .tui_close(Parameters(CloseParams {
            session_id: id.clone(),
        }))
        .await
        .expect("close")
        .0;
    assert!(close.success, "clean quit reaps exit success");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mode_b_runs_yaml_suite() {
    let (handler, _root) = open_handler("mode-b");
    // Mode B resolves relative suites under the handler root; the test writes
    // its own suite with an absolute fixture command (the repo smoke.yaml
    // uses a cargo-relative command valid only under tuilab/).
    let root = scratch_root("mode-b-suite");
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
