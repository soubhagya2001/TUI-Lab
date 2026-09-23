//! HTML report re-rendered from stored JSON results (docs/11 §11.2).
//!
//! Single self-contained file (inline CSS, zero external assets): suite
//! header with counts, per-step timeline rows, failure screens in `<pre>`,
//! diffs escaped. Everything renders from `SuiteResult` — no live data.

use tui_lab_core::{HistoryEntry, SuiteResult};

/// One flaky suite for the report header.
#[derive(Debug, Clone, PartialEq)]
pub struct FlakeSummary {
    /// Suite name.
    pub suite: String,
    /// Recent passes.
    pub passed: usize,
    /// Recent runs considered.
    pub total: usize,
}

/// Summarize flaky suites from history lines: suites with mixed outcomes
/// in the most recent `keep` entries (skips ignored).
pub fn summarize_flakes(history: &[HistoryEntry], keep: usize) -> Vec<FlakeSummary> {
    use std::collections::HashMap;
    let mut recent: HashMap<&str, Vec<bool>> = HashMap::new();
    for entry in history.iter().rev().take(keep) {
        if entry.skipped {
            continue;
        }
        recent
            .entry(entry.suite.as_str())
            .or_default()
            .push(entry.passed);
    }
    let mut out: Vec<FlakeSummary> = recent
        .into_iter()
        .filter(|(_, outcomes)| outcomes.iter().any(|o| *o) && outcomes.iter().any(|o| !o))
        .map(|(suite, outcomes)| FlakeSummary {
            suite: suite.to_string(),
            passed: outcomes.iter().filter(|o| **o).count(),
            total: outcomes.len(),
        })
        .collect();
    out.sort_by(|a, b| a.suite.cmp(&b.suite));
    out
}

/// Render several suite results as one standalone HTML document.
pub fn to_html(results: &[SuiteResult]) -> String {
    to_html_with_flakes(results, &[])
}

/// [`to_html`] plus a flaky-suites header section.
pub fn to_html_with_flakes(results: &[SuiteResult], flakes: &[FlakeSummary]) -> String {
    let mut out = String::from(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<title>TUI Lab report</title>\n<style>\n",
    );
    out.push_str(
        "body{font-family:monospace;max-width:100ch;margin:2em auto;background:#111;color:#ddd}\n\
         h1{font-size:1.2em}.pass{color:#7dffa8}.fail{color:#ff7d7d}\n\
         table{border-collapse:collapse;width:100%}td,th{border:1px solid #444;padding:.2em .5em;text-align:left}\n\
         pre{background:#000;padding:1em;overflow-x:auto;white-space:pre-wrap}\n\
         .tl{position:relative;height:1.2em;background:#000;margin:.2em 0}\n\
         .tl span{position:absolute;top:0;bottom:0;background:#3ddc84;opacity:.8}\n\
         .tl span.bad{background:#ff7d7d}\n",
    );
    out.push_str("</style>\n</head>\n<body>\n<h1>TUI Lab report</h1>\n");
    if !flakes.is_empty() {
        out.push_str("<h2>Flaky suites (recent history)</h2>\n<ul>\n");
        for flake in flakes {
            out.push_str(&format!(
                "<li>{}: {}/{} recent runs passed</li>\n",
                escape(&flake.suite),
                flake.passed,
                flake.total,
            ));
        }
        out.push_str("</ul>\n");
    }
    for result in results {
        let status = if result.passed {
            "<span class=\"pass\">PASS</span>"
        } else {
            "<span class=\"fail\">FAIL</span>"
        };
        out.push_str(&format!(
            "<h2>{status} {}</h2>\n<p>{} steps, {}ms</p>\n",
            escape(&result.suite),
            result.steps.len(),
            result.duration_ms,
        ));
        out.push_str(&waterfall(result));
        out.push_str("<table>\n<tr><th>#</th><th>step</th><th>detail</th><th>ms</th></tr>\n");
        for step in &result.steps {
            let row = if step.passed { "" } else { " class=\"fail\"" };
            out.push_str(&format!(
                "<tr{row}><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                step.index,
                escape(&step.kind),
                escape(&step.detail),
                step.duration_ms,
            ));
        }
        out.push_str("</table>\n");
        if !result.attachments.is_empty() {
            out.push_str("<h3>Attachments</h3>\n<ul>\n");
            for attachment in &result.attachments {
                out.push_str(&format!(
                    "<li><a href=\"{}\">{}</a></li>\n",
                    escape(attachment),
                    escape(attachment),
                ));
            }
            out.push_str("</ul>\n");
        }
        if let Some(failure) = &result.failure {
            out.push_str(&format!(
                "<h3>Failure at step {} ({})</h3>\n<p>expected: {}</p>\n<pre>{}</pre>\n",
                failure.step_index,
                escape(&failure.step),
                escape(&failure.expected),
                escape(&failure.last_screen),
            ));
        }
    }
    out.push_str("</body>\n</html>\n");
    out
}

/// Per-step waterfall bars from `started_ms`/`duration_ms` (B2).
fn waterfall(result: &SuiteResult) -> String {
    if result.steps.is_empty() || result.duration_ms == 0 {
        return String::new();
    }
    let total = result.duration_ms as f64;
    let mut out = String::from("<div class=\"tl\" aria-hidden=\"true\">\n");
    for step in &result.steps {
        let left = (step.started_ms as f64 / total * 100.0).clamp(0.0, 100.0);
        let width = ((step.duration_ms as f64 / total * 100.0).max(0.5)).min(100.0 - left);
        let class = if step.passed { "" } else { " class=\"bad\"" };
        out.push_str(&format!(
            "<span{class} style=\"left:{left:.1}%;width:{width:.1}%\"></span>\n",
        ));
    }
    out.push_str("</div>\n");
    out
}

/// Minimal HTML escape for text and attribute content (S5: sanitize first so
/// control bytes never reach the document; escaping neutralizes markup).
fn escape(text: &str) -> String {
    crate::utils::sanitize(text)
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
