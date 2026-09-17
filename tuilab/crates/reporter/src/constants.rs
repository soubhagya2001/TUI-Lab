//! Report paths and formats (docs/11 §11.2).

/// Default JSON results path (relative to project root).
pub const JSON_REPORT: &str = "reports/results.json";
/// Default JUnit XML path.
pub const JUNIT_REPORT: &str = "reports/junit.xml";
/// Default HTML report path (v2).
pub const HTML_REPORT: &str = "reports/index.html";
/// Raw terminal bytes retained per failure bundle (docs/11 §11.3 item 4).
pub const MAX_RAW_BYTES: usize = 64 * 1_024;
