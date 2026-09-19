//! Parallel fan-out: order preserved, work overlaps, failures contained.

use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use tui_lab_core::{run_suites, RunOptions};
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

fn quick_suite(bin: &str, name: &str) -> TestFile {
    TestFile::from_yaml(&format!(
        r#"schema: tui-lab/v1
name: {name}
application:
  command: "{bin}"
steps:
  - wait_for_text:
      text: "TUI-LAB-SAMPLE"
  - press: q
assertions:
  - exit_code: 0
"#
    ))
    .expect("parse suite")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn parallel_suites_overlap_and_stay_ordered() {
    let bin = fixture_bin();
    let files = vec![
        quick_suite(&bin, "aaa-first"),
        quick_suite(&bin, "bbb-second"),
        quick_suite(&bin, "ccc-third"),
    ];
    let opts = RunOptions {
        wait_default: Duration::from_secs(15),
        ..RunOptions::default()
    };

    let started = Instant::now();
    let results = run_suites(files, &opts, 3).await;
    let wall_ms = started.elapsed().as_millis() as u64;

    // All green.
    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|result| result.passed));

    // Input order preserved regardless of completion order.
    let names: Vec<&str> = results.iter().map(|result| result.suite.as_str()).collect();
    assert_eq!(names, ["aaa-first", "bbb-second", "ccc-third"]);

    // Overlap proof, self-referential (no absolute thresholds to flake on
    // slow runners): concurrent wall time beats the summed durations.
    let summed: u64 = results.iter().map(|result| result.duration_ms).sum();
    assert!(
        (wall_ms as f64) < 0.9 * (summed.max(1) as f64),
        "wall {wall_ms}ms should overlap summed {summed}ms"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn one_failure_does_not_abort_siblings() {
    let bin = fixture_bin();
    let files = vec![
        quick_suite(&bin, "green-a"),
        {
            let mut bad = quick_suite(&bin, "red-b");
            bad.steps.push(tui_lab_protocol::Step::AssertText(
                tui_lab_protocol::TextAssertion {
                    contains: Some("no-such-screen".to_string()),
                    not_contains: None,
                    regex: None,
                },
            ));
            bad
        },
        quick_suite(&bin, "green-c"),
    ];
    let results = run_suites(files, &RunOptions::default(), 3).await;
    assert_eq!(results.len(), 3);
    assert!(results[0].passed);
    assert!(!results[1].passed);
    assert!(results[2].passed);
    assert!(results[1].failure.is_some());
}
