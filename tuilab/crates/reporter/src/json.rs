//! JSON results: write the full trace, read it back (docs/11 §11.2).
//!
//! `reports/results.json` is the source of truth; JUnit/HTML re-render from
//! it so formats can never disagree.

use std::path::Path;

use tui_lab_core::SuiteResult;

use crate::error::{ReporterError, Result};

/// Serialize a suite result to pretty JSON.
pub fn to_json(result: &SuiteResult) -> Result<String> {
    serde_json::to_string_pretty(result)
        .map_err(|e| ReporterError::Message(format!("encode results: {e}")))
}

/// Write `reports/results.json`, creating parent directories.
pub fn write_json(path: &Path, result: &SuiteResult) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ReporterError::Io(format!("create {}: {e}", parent.display())))?;
    }
    let text = to_json(result)?;
    std::fs::write(path, text)
        .map_err(|e| ReporterError::Io(format!("write {}: {e}", path.display())))?;
    Ok(())
}

/// Load a previously written `reports/results.json`.
pub fn load_json(path: &Path) -> Result<SuiteResult> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| ReporterError::Io(format!("read {}: {e}", path.display())))?;
    serde_json::from_str(&text).map_err(|e| ReporterError::Message(format!("decode results: {e}")))
}

/// Write several suite results as a JSON array.
pub fn write_json_all(path: &Path, results: &[SuiteResult]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ReporterError::Io(format!("create {}: {e}", parent.display())))?;
    }
    let text = serde_json::to_string_pretty(results)
        .map_err(|e| ReporterError::Message(format!("encode results: {e}")))?;
    std::fs::write(path, text)
        .map_err(|e| ReporterError::Io(format!("write {}: {e}", path.display())))?;
    Ok(())
}

/// Load a JSON array written by [`write_json_all`].
pub fn load_json_all(path: &Path) -> Result<Vec<SuiteResult>> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| ReporterError::Io(format!("read {}: {e}", path.display())))?;
    serde_json::from_str(&text).map_err(|e| ReporterError::Message(format!("decode results: {e}")))
}
