//! Async SDK flow against the Ratatui fixture (mirrors the Python suite).

use std::path::PathBuf;
use std::process::Command;

use tui_lab_sdk::{find_binary, LaunchOptions, Runner, TuiLabError, TuiTest};

/// Build the fixture binary on demand; return its path with forward slashes
/// (safe inside YAML double-quoted scalars on Windows).
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fixture_flow() {
    let mut tui = TuiTest::launch(fixture_bin(), LaunchOptions::new())
        .await
        .expect("launch");
    tui.expect_text("TUI-LAB-SAMPLE", 10_000, false)
        .await
        .expect("boot");
    tui.press("DOWN").await.expect("press");
    tui.press("ENTER").await.expect("press");
    let screen = tui
        .expect_text("Selected: beta-chair", 10_000, false)
        .await
        .expect("detail");
    assert!(screen.contains("beta-chair"));
    tui.resize(80, 24).await.expect("resize");
    tui.press("ESC").await.expect("press");
    tui.expect_text("TUI-LAB-SAMPLE", 10_000, false)
        .await
        .expect("list");
    tui.press("/").await.expect("press");
    tui.type_text("table", false).await.expect("type");
    tui.expect_text("Search: table", 10_000, false)
        .await
        .expect("search");
    tui.expect_not_text("beta-chair").await.expect("filter");
    let snap = tui.snapshot("rs-proof").await.expect("snapshot");
    assert_eq!(snap.get("ok"), Some(&serde_json::Value::Bool(true)));
    tui.close().await.expect("close");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unknown_key_raises() {
    let mut tui = TuiTest::launch(fixture_bin(), LaunchOptions::new())
        .await
        .expect("launch");
    tui.expect_text("TUI-LAB-SAMPLE", 10_000, false)
        .await
        .expect("boot");
    assert!(tui.press("F13").await.is_err());
    tui.close().await.expect("close");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn wait_timeout_carries_evidence() {
    let mut tui = TuiTest::launch(fixture_bin(), LaunchOptions::new())
        .await
        .expect("launch");
    let err = tui
        .expect_text("no-such-screen", 500, false)
        .await
        .expect_err("timeout");
    let _: &TuiLabError = &err;
    assert!(err.detail.is_some());
    tui.close().await.expect("close");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn runner_runs_yaml() {
    let dir = std::env::temp_dir().join(format!("tuilab-rs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let suite = dir.join("mini.yaml");
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
    let results = Runner::run(&suite, None).await.expect("run suite");
    assert!(!results.is_empty());
    assert!(results
        .iter()
        .all(|suite| suite.get("passed") == Some(&serde_json::Value::Bool(true))));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn find_binary_resolves() {
    assert!(find_binary(None).is_ok());
}
