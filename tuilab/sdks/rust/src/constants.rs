//! SDK-facing constants (docs/09).

/// Default report directory (the engine's `reports/`, rebased by
/// `Runner::run_with_reports_dir`).
pub const DEFAULT_REPORTS_DIR: &str = "reports";
/// Results file written inside a report directory.
pub const RESULTS_FILE: &str = "results.json";
