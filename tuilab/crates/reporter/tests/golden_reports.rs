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
        skipped: false,
        attempts: 1,
        exit_success: Some(false),
        exit_signal: None,
        exit_code: Some(1),
        duration_ms: 1500,
        steps: vec![
            StepResult {
                index: 0,
                kind: "wait_for_text \"Main\"".to_string(),
                passed: true,
                detail: "found".to_string(),
                duration_ms: 120,
                started_ms: 0,
            },
            StepResult {
                index: 1,
                kind: "assert_text".to_string(),
                passed: false,
                detail: "expected visible text \"Dash\"".to_string(),
                duration_ms: 30,
                started_ms: 120,
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
        trace: Vec::new(),
        trace_truncated: false,
        attachments: Vec::new(),
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
fn hostile_screen_is_escaped_and_sanitized() {
    // S5: a screen carrying markup, entities, and control bytes must come
    // out inert in both sinks — no raw tags, no NUL/control bytes.
    let mut result = fixture_result();
    if let Some(failure) = result.failure.as_mut() {
        failure.last_screen = "oops <script>alert(&1)</script>\x00\x07\x1b[2J done".to_string();
    }
    let xml = to_junit(&result);
    assert!(!xml.contains("<script>"), "JUnit must escape markup");
    assert!(!xml.contains('\x00'), "JUnit must strip NUL");
    assert!(!xml.contains('\x07'), "JUnit must strip BEL");
    assert!(xml.contains("&lt;script&gt;"), "escaped form present");
    let html = to_html(&[result]);
    assert!(!html.contains("<script>alert"), "HTML must escape markup");
    assert!(!html.contains('\x00'), "HTML must strip NUL");
    assert!(html.contains("&lt;script&gt;"), "escaped form present");
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

#[test]
fn pre_9c_results_without_exit_code_still_parse() {
    // Additive wire rule: old results.json files (no exit_code field) load
    // with None rather than failing strict deserialization.
    let old = serde_json::json!({
        "schema": "tui-lab/v1",
        "suite": "legacy",
        "passed": true,
        "exit_success": true,
        "exit_signal": null,
        "duration_ms": 10,
        "steps": [],
        "failure": null,
        "terminal": {"width": 80, "height": 24, "term": "xterm-256color"},
    });
    let result: SuiteResult = serde_json::from_value(old).expect("old shape parses");
    assert_eq!(result.exit_code, None);
}

#[test]
fn html_waterfall_and_attachments_render() {
    use tui_lab_reporter::{to_html_with_flakes, FlakeSummary};
    let mut result = fixture_result();
    result.attachments = vec!["attachments/smoke/app.log".to_string()];
    let html = to_html_with_flakes(
        &[result],
        &[FlakeSummary {
            suite: "smoke & <mirrors>".to_string(),
            passed: 1,
            total: 3,
        }],
    );
    assert!(html.contains("class=\"tl\""), "waterfall timeline");
    assert!(
        html.contains("attachments/smoke/app.log"),
        "attachment link"
    );
    assert!(html.contains("Flaky suites"), "flake section");
    assert!(html.contains("1/3 recent runs passed"), "flake counts");
}

#[test]
fn flake_summary_needs_mixed_outcomes() {
    use tui_lab_core::HistoryEntry;
    use tui_lab_reporter::summarize_flakes;
    let history = vec![
        HistoryEntry {
            ts: 1,
            suite: "steady".into(),
            passed: true,
            skipped: false,
            attempts: 1,
            duration_ms: 10,
        },
        HistoryEntry {
            ts: 2,
            suite: "steady".into(),
            passed: true,
            skipped: false,
            attempts: 1,
            duration_ms: 10,
        },
        HistoryEntry {
            ts: 3,
            suite: "flaky".into(),
            passed: true,
            skipped: false,
            attempts: 1,
            duration_ms: 10,
        },
        HistoryEntry {
            ts: 4,
            suite: "flaky".into(),
            passed: false,
            skipped: false,
            attempts: 2,
            duration_ms: 10,
        },
        HistoryEntry {
            ts: 5,
            suite: "skipped".into(),
            passed: true,
            skipped: true,
            attempts: 1,
            duration_ms: 0,
        },
    ];
    let flakes = summarize_flakes(&history, 30);
    assert_eq!(flakes.len(), 1);
    assert_eq!(flakes[0].suite, "flaky");
    assert_eq!((flakes[0].passed, flakes[0].total), (1, 2));
}
