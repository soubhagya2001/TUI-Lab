//! Golden reports: fixed results in, stable JSON + valid JUnit out.

use tui_lab_core::{FailureInfo, StepResult, SuiteResult, TerminalInfo};
use tui_lab_reporter::{
    load_json, load_json_all, to_html, to_json, to_junit, to_junit_all, write_json, write_json_all,
};

fn fixture_result() -> SuiteResult {
    SuiteResult {
        schema: "tui-lab/v1".to_string(),
        suite: "smoke & <mirrors>".to_string(),
        passed: false,
        exit_success: Some(false),
        exit_signal: None,
        duration_ms: 1500,
        steps: vec![
            StepResult {
                index: 0,
                kind: "wait_for_text \"Main\"".to_string(),
                passed: true,
                detail: "found".to_string(),
                duration_ms: 120,
            },
            StepResult {
                index: 1,
                kind: "assert_text".to_string(),
                passed: false,
                detail: "expected visible text \"Dash\"".to_string(),
                duration_ms: 30,
            },
        ],
        failure: Some(FailureInfo {
            step_index: 1,
            step: "assert_text".to_string(),
            expected: "expected visible text \"Dash\"".to_string(),
            actual: "Main screen".to_string(),
            last_screen: "Main screen\n".to_string(),
            input_history: vec!["press ENTER".to_string()],
        }),
        terminal: TerminalInfo {
            width: 120,
            height: 40,
            term: "xterm-256color".to_string(),
        },
    }
}

#[test]
fn json_round_trip_is_stable() {
    let result = fixture_result();
    let first = to_json(&result).expect("encode");
    let back: SuiteResult = serde_json::from_str(&first).expect("decode");
    assert_eq!(result, back);
    let second = to_json(&back).expect("re-encode");
    assert_eq!(first, second, "JSON output is deterministic");
}

#[test]
fn json_file_round_trip() {
    let dir = std::env::temp_dir().join("tuilab-report-golden");
    let path = dir.join("results.json");
    write_json(&path, &fixture_result()).expect("write");
    let back = load_json(&path).expect("read");
    assert_eq!(back.suite, "smoke & <mirrors>");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn junit_counts_failures_and_escapes() {
    let xml = to_junit(&fixture_result());
    assert!(xml.contains("tests=\"2\""), "two cases");
    assert!(xml.contains("failures=\"1\""), "one failure");
    // XML escaping: raw & and < must not appear.
    assert!(xml.contains("smoke &amp; &lt;mirrors&gt;"));
    assert!(xml.contains("<failure message="));
    assert!(xml.contains("step[1] assert_text"));
    // Failure detail + evidence land in the report.
    assert!(xml.contains("expected visible text"));
    assert!(xml.contains("<system-err>"));
}

#[test]
fn junit_multi_suite_wraps_testsuites() {
    let first = fixture_result();
    let mut second = fixture_result();
    second.suite = "second".to_string();
    second.passed = true;
    second.failure = None;
    for step in &mut second.steps {
        step.passed = true;
    }
    let dir = std::env::temp_dir().join("tuilab-report-multi");
    let path = dir.join("results.json");
    write_json_all(&path, &[first, second]).expect("write all");
    let back = load_json_all(&path).expect("read all");
    assert_eq!(back.len(), 2);
    let xml = to_junit_all(&back);
    assert!(xml.contains("<testsuites>"));
    assert!(xml.contains("</testsuites>"));
    assert_eq!(xml.matches("<testsuite ").count(), 2);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn junit_passing_suite_has_no_failures() {
    let mut result = fixture_result();
    result.passed = true;
    result.failure = None;
    for step in &mut result.steps {
        step.passed = true;
        step.detail = "ok".to_string();
    }
    let xml = to_junit(&result);
    assert!(xml.contains("failures=\"0\""));
    assert!(!xml.contains("<failure"));
}

#[test]
fn html_renders_offline_report_with_escapes() {
    let html = to_html(&[fixture_result()]);
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("smoke &amp; &lt;mirrors&gt;"));
    assert!(html.contains("FAIL"));
    assert!(html.contains("<pre>"));
    assert!(!html.contains("http"), "no external assets");
}

#[test]
fn html_passing_suite_marks_pass() {
    let mut result = fixture_result();
    result.passed = true;
    result.failure = None;
    let html = to_html(&[result]);
    assert!(html.contains("PASS"));
    assert!(!html.contains("<pre>"));
}
