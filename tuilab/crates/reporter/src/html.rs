//! HTML report re-rendered from stored JSON results (docs/11 §11.2).
//!
//! Single self-contained file (inline CSS, zero external assets): suite
//! header with counts, per-step timeline rows, failure screens in `<pre>`,
//! diffs escaped. Everything renders from `SuiteResult` — no live data.

use tui_lab_core::SuiteResult;

/// Render several suite results as one standalone HTML document.
pub fn to_html(results: &[SuiteResult]) -> String {
    let mut out = String::from(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<title>TUI Lab report</title>\n<style>\n",
    );
    out.push_str(
        "body{font-family:monospace;max-width:100ch;margin:2em auto;background:#111;color:#ddd}\n\
         h1{font-size:1.2em}.pass{color:#7dffa8}.fail{color:#ff7d7d}\n\
         table{border-collapse:collapse;width:100%}td,th{border:1px solid #444;padding:.2em .5em;text-align:left}\n\
         pre{background:#000;padding:1em;overflow-x:auto;white-space:pre-wrap}\n",
    );
    out.push_str("</style>\n</head>\n<body>\n<h1>TUI Lab report</h1>\n");
    for result in results {
        let status = if result.passed {
            "<span class=\"pass\">PASS</span>"
        } else {
            "<span class=\"fail\">FAIL</span>"
        };
        out.push_str(&format!(
            "<h2>{status} {}</h2>\n<p>{} steps, {}ms</p>\n<table>\n<tr><th>#</th><th>step</th><th>detail</th><th>ms</th></tr>\n",
            escape(&result.suite),
            result.steps.len(),
            result.duration_ms,
        ));
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

/// Minimal HTML escape for text and attribute content.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
