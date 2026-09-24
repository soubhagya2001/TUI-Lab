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
    tui.expect_text("TUI-LAB-SAMPLE", 10_000, false, None)
        .await
        .expect("boot");
    tui.press("DOWN").await.expect("press");
    tui.press("ENTER").await.expect("press");
    let screen = tui
        .expect_text("Selected: beta-chair", 10_000, false, None)
        .await
        .expect("detail");
    assert!(screen.contains("beta-chair"));
    tui.resize(80, 24).await.expect("resize");
    tui.press("ESC").await.expect("press");
    tui.expect_text("TUI-LAB-SAMPLE", 10_000, false, None)
        .await
        .expect("list");
    tui.press("/").await.expect("press");
    tui.type_text("table", false).await.expect("type");
    tui.expect_text("Search: table", 10_000, false, None)
        .await
        .expect("search");
    tui.expect_not_text("beta-chair").await.expect("filter");
    let snap = tui.snapshot("rs-proof").await.expect("snapshot");
    assert_eq!(snap.get("ok"), Some(&serde_json::Value::Bool(true)));
    tui.close(None, None).await.expect("close");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unknown_key_raises() {
    let mut tui = TuiTest::launch(fixture_bin(), LaunchOptions::new())
        .await
        .expect("launch");
    tui.expect_text("TUI-LAB-SAMPLE", 10_000, false, None)
        .await
        .expect("boot");
    assert!(tui.press("F13").await.is_err());
    tui.close(None, None).await.expect("close");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn wait_timeout_carries_evidence() {
    let mut tui = TuiTest::launch(fixture_bin(), LaunchOptions::new())
        .await
        .expect("launch");
    let err = tui
        .expect_text("no-such-screen", 500, false, None)
        .await
        .expect_err("timeout");
    let _: &TuiLabError = &err;
    assert!(err.detail.is_some());
    tui.close(None, None).await.expect("close");
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn runner_isolates_concurrent_runs() {
    // K3: two runs share one CWD but keep their own report directories.
    let dir = std::env::temp_dir().join(format!("tuilab-rs-out-{}", std::process::id()));
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
    let first = dir.join("out-a");
    let second = dir.join("out-b");
    let (left, right) = tokio::join!(
        Runner::run_with_reports_dir(&suite, None, Some(&first)),
        Runner::run_with_reports_dir(&suite, None, Some(&second)),
    );
    for results in [left.expect("first run"), right.expect("second run")] {
        assert!(!results.is_empty());
        assert!(results
            .iter()
            .all(|suite| suite.get("passed") == Some(&serde_json::Value::Bool(true))));
    }
    assert!(first.join("results.json").is_file());
    assert!(second.join("results.json").is_file());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn launch_options_default_is_usable_geometry() {
    // K2: Default must give a real terminal, never 0x0.
    let opts = LaunchOptions::default();
    assert_eq!((opts.width, opts.height), (120, 40));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn assert_condition_roundtrip() {
    // K1: generic condition passthrough.
    let mut tui = TuiTest::launch(fixture_bin(), LaunchOptions::new())
        .await
        .expect("launch");
    tui.expect_text("TUI-LAB-SAMPLE", 10_000, false, None)
        .await
        .expect("boot");
    let passed = tui
        .assert(serde_json::json!({"type": "text_visible", "text": "TUI-LAB-SAMPLE"}))
        .await
        .expect("assert");
    assert_eq!(passed.get("passed"), Some(&serde_json::Value::Bool(true)));
    let failed = tui
        .assert(serde_json::json!({"type": "text_visible", "text": "no-such-screen"}))
        .await
        .expect("assert assessed");
    assert_eq!(failed.get("passed"), Some(&serde_json::Value::Bool(false)));
    assert!(tui
        .assert(serde_json::json!({"type": "no-such-condition"}))
        .await
        .is_err());
    tui.close(None, None).await.expect("close");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn styled_screen_and_quit_close() {
    // K1: styled screens carry cells; close(quit) exits cleanly.
    let mut tui = TuiTest::launch(fixture_bin(), LaunchOptions::new())
        .await
        .expect("launch");
    tui.expect_text("TUI-LAB-SAMPLE", 10_000, false, Some(25))
        .await
        .expect("boot");
    let styled = tui.screen(true, false).await.expect("styled screen");
    let cells = styled.get("cells").and_then(serde_json::Value::as_array);
    assert!(cells.is_some_and(|cells| !cells.is_empty()));
    // E1: the tree dump is a JSON array (possibly empty on plain screens).
    let tree = tui.screen(false, true).await.expect("tree screen");
    assert!(tree.get("tree").and_then(|t| t.as_array()).is_some());
    let out = tui.close(Some("q"), None).await.expect("quit close");
    assert_eq!(out.get("ok"), Some(&serde_json::Value::Bool(true)));
}

#[test]
fn read_results_rejects_stale_files() {
    // K3: missing or older-than-run output never parses.
    let dir = std::env::temp_dir().join(format!("tuilab-rs-stale-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch");
    let target = dir.join("results.json");
    assert!(
        Runner::read_results(target.to_str().expect("utf8"), std::time::SystemTime::now()).is_err()
    );
    std::fs::write(&target, "[]").expect("stale file");
    set_mtime_back(&target, 60);
    assert!(
        Runner::read_results(target.to_str().expect("utf8"), std::time::SystemTime::now()).is_err()
    );
    std::fs::write(&target, r#"[{"passed": true}]"#).expect("fresh file");
    let results = Runner::read_results(
        target.to_str().expect("utf8"),
        std::time::SystemTime::now() - std::time::Duration::from_secs(1),
    )
    .expect("fresh parses");
    assert_eq!(results.len(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Backdate a file's mtime by `secs` (no extra deps: powershell/touch).
fn set_mtime_back(path: &std::path::Path, secs: u64) {
    #[cfg(windows)]
    let status = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!(
                "(Get-Item \"{}\").LastWriteTime = (Get-Date).AddSeconds(-{secs})",
                path.display()
            ),
        ])
        .status()
        .expect("powershell");
    #[cfg(not(windows))]
    let status = std::process::Command::new("touch")
        .args([
            "-d",
            &format!("{secs} seconds ago"),
            &path.to_string_lossy(),
        ])
        .status()
        .expect("touch");
    assert!(status.success(), "backdate mtime");
}
