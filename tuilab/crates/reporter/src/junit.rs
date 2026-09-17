//! JUnit XML re-rendered from stored JSON results (docs/11 §11.2).
//!
//! Hand-rolled emission (no XML crate): the schema surface is tiny
//! (`testsuite`/`testcase`/`failure`) and every byte stays under our control.
//! If real-world CI servers reject the output, adopt `quick-xml` then.

use tui_lab_core::SuiteResult;

/// Render `SuiteResult` as JUnit XML.
///
/// Step-indexed case names (`step[3] press DOWN`); failure text carries the
/// expected-vs-actual detail plus the last screen tail.
pub fn to_junit(result: &SuiteResult) -> String {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let failures = result.steps.iter().filter(|step| !step.passed).count();
    out.push_str(&format!(
        "<testsuite name=\"{}\" tests=\"{}\" failures=\"{}\" time=\"{:.3}\">\n",
        escape(&result.suite),
        result.steps.len(),
        failures,
        result.duration_ms as f64 / 1000.0,
    ));
    for step in &result.steps {
        out.push_str(&format!(
            "  <testcase name=\"step[{}] {}\" time=\"{:.3}\"",
            step.index,
            escape(&step.kind),
            step.duration_ms as f64 / 1000.0,
        ));
        if step.passed {
            out.push_str("/>\n");
        } else {
            out.push_str(&format!(
                ">\n    <failure message=\"{}\">{}</failure>\n  </testcase>\n",
                escape(step.detail.lines().next().unwrap_or("step failed")),
                escape(&step.detail)
            ));
        }
    }
    if !result.passed {
        if let Some(failure) = &result.failure {
            out.push_str(&format!(
                "  <system-err>step {} ({}): expected {} | actual screen:\n{}</system-err>\n",
                failure.step_index,
                escape(&failure.step),
                escape(&failure.expected),
                escape(&failure.last_screen),
            ));
        }
    }
    out.push_str("</testsuite>\n");
    out
}

/// Render several suite results as `<testsuites>`.
pub fn to_junit_all(results: &[SuiteResult]) -> String {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<testsuites>\n");
    for result in results {
        let single = to_junit(result);
        let body = single.lines().skip(1).collect::<Vec<_>>().join("\n");
        out.push_str(&body);
        out.push('\n');
    }
    out.push_str("</testsuites>\n");
    out
}

/// Minimal XML escape for attribute and element text.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
