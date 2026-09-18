//! CLI end to end: init scaffolds, run executes, bad input exits 2.
//!
//! Drives the built `tuilab` binary as a subprocess in scratch directories.

use std::path::{Path, PathBuf};
use std::process::Command;

fn tuilab() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tuilab"))
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

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tuilab-cli-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, text).expect("write suite");
    path
}

fn green_suite(bin: &str) -> String {
    format!(
        r#"schema: tui-lab/v1
name: green
application:
  command: "{bin}"
terminal:
  width: 120
  height: 40
steps:
  - wait_for_text:
      text: "TUI-LAB-SAMPLE"
  - press: q
assertions:
  - exit_code: 0
"#
    )
}

#[test]
fn init_scaffolds_config_and_smoke() {
    let dir = scratch("init");
    let output = Command::new(tuilab())
        .arg("init")
        .current_dir(&dir)
        .output()
        .expect("run init");
    assert!(output.status.success());
    assert!(dir.join("tuilab.yaml").is_file());
    assert!(dir.join("tests").join("smoke.yaml").is_file());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn run_green_suite_exits_zero_with_results() {
    let dir = scratch("green");
    let suite = write(&dir, "green.yaml", &green_suite(&fixture_bin()));
    let output = Command::new(tuilab())
        .arg("run")
        .arg(&suite)
        .current_dir(&dir)
        .output()
        .expect("run suite");
    assert!(
        output.status.success(),
        "stdout:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let results = dir.join("reports").join("results.json");
    assert!(results.is_file(), "results.json written");
    let text = std::fs::read_to_string(&results).expect("read results");
    assert!(text.contains("\"passed\": true"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn run_failing_suite_exits_one() {
    let dir = scratch("failing");
    let suite = write(
        &dir,
        "failing.yaml",
        &format!(
            r#"schema: tui-lab/v1
name: failing
application:
  command: "{}"
steps:
  - wait_for_text:
      text: "TUI-LAB-SAMPLE"
  - assert_text:
      contains: "no-such-screen"
  - press: q
"#,
            fixture_bin()
        ),
    );
    let output = Command::new(tuilab())
        .arg("run")
        .arg(&suite)
        .current_dir(&dir)
        .output()
        .expect("run suite");
    assert_eq!(output.status.code(), Some(1));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn run_bad_yaml_exits_two_without_launch() {
    let dir = scratch("bad");
    let suite = write(&dir, "bad.yaml", "schema: [unclosed\n");
    let output = Command::new(tuilab())
        .arg("run")
        .arg(&suite)
        .current_dir(&dir)
        .output()
        .expect("run suite");
    assert_eq!(output.status.code(), Some(2));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn report_renders_junit_from_results() {
    let dir = scratch("report");
    let suite = write(&dir, "green.yaml", &green_suite(&fixture_bin()));
    let run = Command::new(tuilab())
        .arg("run")
        .arg(&suite)
        .current_dir(&dir)
        .output()
        .expect("run suite");
    assert!(run.status.success());
    let out = dir.join("junit.xml");
    let report = Command::new(tuilab())
        .arg("report")
        .arg("--format")
        .arg("junit")
        .arg("--out")
        .arg(&out)
        .current_dir(&dir)
        .output()
        .expect("report");
    assert!(report.status.success());
    let xml = std::fs::read_to_string(&out).expect("read junit");
    assert!(xml.contains("failures=\"0\""));
    let _ = std::fs::remove_dir_all(&dir);
}
