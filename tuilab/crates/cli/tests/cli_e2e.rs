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
fn run_parallel_dir_reports_both_suites() {
    let dir = scratch("parallel");
    let bin = fixture_bin();
    write(
        &dir,
        "a.yaml",
        &green_suite(&bin).replace("name: green", "name: par-a"),
    );
    write(
        &dir,
        "b.yaml",
        &green_suite(&bin).replace("name: green", "name: par-b"),
    );
    let output = Command::new(tuilab())
        .arg("run")
        .arg(&dir)
        .arg("--parallel")
        .arg("2")
        .current_dir(&dir)
        .output()
        .expect("run suites");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stdout:\n{stdout}");
    assert!(stdout.contains("par-a") && stdout.contains("par-b"));
    assert!(stdout.contains("2 passed, 0 failed"), "stdout:\n{stdout}");
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
    let html_out = dir.join("index.html");
    let html = Command::new(tuilab())
        .arg("report")
        .arg("--format")
        .arg("html")
        .arg("--out")
        .arg(&html_out)
        .current_dir(&dir)
        .output()
        .expect("report html");
    assert!(html.status.success());
    let page = std::fs::read_to_string(&html_out).expect("read html");
    assert!(page.contains("<!DOCTYPE html>"));
    assert!(page.contains("green"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn run_step_mode_advances_on_piped_enters() {
    use std::io::Write as _;
    use std::process::Stdio;

    let dir = scratch("step");
    let suite = write(&dir, "green.yaml", &green_suite(&fixture_bin()));
    // Piped Enters auto-continue every pause (q would abort instead).
    let mut child = Command::new(tuilab())
        .arg("run")
        .arg(&suite)
        .arg("--step")
        .current_dir(&dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("run suite");
    child
        .stdin
        .as_mut()
        .expect("step stdin")
        .write_all(b"\n\n\n\n")
        .expect("send enters");
    let output = child.wait_with_output().expect("step run");
    assert!(
        output.status.success(),
        "stdout:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("to continue"), "step prompts shown");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn config_default_timeout_wires_into_waits() {
    // C5: `default_timeout` from tuilab.yaml becomes the wait default — a
    // stepless wait on absent text fails in ~100ms, not the 10s fallback.
    use std::time::{Duration, Instant};

    let dir = scratch("wired-timeout");
    std::fs::write(dir.join("tuilab.yaml"), "default_timeout: 100ms\n").expect("config");
    let suite = write(
        &dir,
        "waiting.yaml",
        &format!(
            "schema: tui-lab/v1\nname: waiting\napplication:\n  command: \"{}\"\nsteps:\n  - wait_for_text:\n      text: \"text-that-never-appears\"\n  - press: q\ncleanup:\n  - press: q\n",
            fixture_bin()
        ),
    );
    let started = Instant::now();
    let output = Command::new(tuilab())
        .arg("run")
        .arg(&suite)
        .current_dir(&dir)
        .output()
        .expect("run suite");
    assert_eq!(output.status.code(), Some(1), "wait failure exits 1");
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "wired 100ms timeout must fail fast, took {:?}",
        started.elapsed()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn bad_default_timeout_is_config_error() {
    // C5: a malformed default_timeout is exit 2, never a silent default.
    let dir = scratch("bad-timeout");
    std::fs::write(dir.join("tuilab.yaml"), "default_timeout: forever\n").expect("config");
    let suite = write(&dir, "green.yaml", &green_suite(&fixture_bin()));
    let output = Command::new(tuilab())
        .arg("run")
        .arg(&suite)
        .current_dir(&dir)
        .output()
        .expect("run suite");
    assert_eq!(output.status.code(), Some(2), "bad timeout exits 2");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn oversized_parallel_warns_and_clamps() {
    // R9: `--parallel 99` warns on stderr, clamps to the cap, still runs.
    let dir = scratch("clamp");
    let suite = write(&dir, "green.yaml", &green_suite(&fixture_bin()));
    let output = Command::new(tuilab())
        .arg("run")
        .arg(&suite)
        .arg("--parallel")
        .arg("99")
        .current_dir(&dir)
        .output()
        .expect("run suite");
    assert!(output.status.success(), "clamped run still passes");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("clamped"), "warns on stderr: {stderr}");
    let _ = std::fs::remove_dir_all(&dir);
}

fn tagged_suite(bin: &str, name: &str, tags: &str) -> String {
    let tags_block = if tags.is_empty() {
        String::new()
    } else {
        format!("tags:\n{tags}")
    };
    format!(
        "schema: tui-lab/v1\nname: {name}\napplication:\n  command: \"{bin}\"\n{tags_block}steps:\n  - wait_for_text:\n      text: \"TUI-LAB-SAMPLE\"\n  - press: q\nassertions:\n  - exit_code: 0\n",
    )
}

#[test]
fn tag_filter_runs_matches_and_counts_skips() {
    // Phase A: --tags keeps matching suites; the rest report skipped.
    let dir = scratch("tags");
    let bin = fixture_bin();
    write(&dir, "a.yaml", &tagged_suite(&bin, "tagged", "  - smoke\n"));
    write(&dir, "b.yaml", &tagged_suite(&bin, "plain", ""));
    let output = Command::new(tuilab())
        .arg("run")
        .arg(&dir)
        .arg("--tags")
        .arg("smoke")
        .current_dir(&dir)
        .output()
        .expect("run suite");
    assert!(output.status.success(), "no failures");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("1 passed, 0 failed, 1 skipped"),
        "stdout:\n{stdout}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn shards_partition_suites() {
    // Phase A: 1/2 + 2/2 cover everything exactly once.
    let dir = scratch("shards");
    let bin = fixture_bin();
    write(
        &dir,
        "a.yaml",
        &green_suite(&bin).replace("name: green", "name: shard-a"),
    );
    write(
        &dir,
        "b.yaml",
        &green_suite(&bin).replace("name: green", "name: shard-b"),
    );
    let mut seen = Vec::new();
    for shard in ["1/2", "2/2"] {
        let output = Command::new(tuilab())
            .arg("run")
            .arg(&dir)
            .arg("--shard")
            .arg(shard)
            .current_dir(&dir)
            .output()
            .expect("run suite");
        assert!(output.status.success(), "shard {shard} passes");
        seen.push(String::from_utf8_lossy(&output.stdout).into_owned());
    }
    assert!(
        seen[0].contains("shard-a") && !seen[0].contains("shard-b"),
        "{}",
        seen[0]
    );
    assert!(
        seen[1].contains("shard-b") && !seen[1].contains("shard-a"),
        "{}",
        seen[1]
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn retries_record_attempts() {
    // Phase A: a failing suite with --retries 1 runs twice (attempts == 2).
    let dir = scratch("retries");
    let suite = write(
        &dir,
        "failing.yaml",
        &format!(
            "schema: tui-lab/v1\nname: flaky\napplication:\n  command: \"{}\"\nsteps:\n  - wait_for_text:\n      text: \"TUI-LAB-SAMPLE\"\n  - assert_text:\n      contains: \"no-such-screen\"\n  - press: q\n",
            fixture_bin()
        ),
    );
    let output = Command::new(tuilab())
        .arg("run")
        .arg(&suite)
        .arg("--retries")
        .arg("1")
        .current_dir(&dir)
        .output()
        .expect("run suite");
    assert_eq!(output.status.code(), Some(1), "still fails");
    let text = std::fs::read_to_string(dir.join("reports").join("results.json")).expect("results");
    assert!(text.contains("\"attempts\": 2"), "two attempts recorded");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn skipped_suite_never_fails() {
    // Phase A: skip:true reports skipped with exit 0.
    let dir = scratch("skipped");
    let bin = fixture_bin();
    write(
        &dir,
        "skip.yaml",
        &format!(
            "schema: tui-lab/v1\nname: skipped\napplication:\n  command: \"{bin}\"\nskip: true\nsteps:\n  - wait_for_text:\n      text: \"TUI-LAB-SAMPLE\"\n"
        ),
    );
    let output = Command::new(tuilab())
        .arg("run")
        .arg(&dir)
        .current_dir(&dir)
        .output()
        .expect("run suite");
    assert!(output.status.success(), "skips exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("0 passed, 0 failed, 1 skipped"),
        "stdout:\n{stdout}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
