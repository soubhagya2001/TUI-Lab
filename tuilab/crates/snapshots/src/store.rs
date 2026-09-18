//! Golden snapshot store: text + cell-JSON save/load/compare (docs/06 §6.2–6.3).
//!
//! Masking: regex entries replace volatile text before comparison; entries
//! starting with `region:` are rejected with a clear error until v2 region
//! support lands. Nothing here touches a PTY — callers feed screen dumps.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Result, SnapshotError};
use crate::utils::{snapshot_file, unified_diff};

/// Placeholder substituted for masked spans.
pub const MASK_REPLACEMENT: &str = "<masked>";

/// One styled cell (docs/06 §6.2 render snapshot).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CellData {
    /// Column.
    pub x: u16,
    /// Row.
    pub y: u16,
    /// Grapheme.
    pub char: String,
    /// Foreground color name.
    pub fg: String,
    /// Background color name.
    pub bg: String,
    /// Bold flag.
    #[serde(default)]
    pub bold: bool,
    /// Underline flag.
    #[serde(default)]
    pub underline: bool,
    /// Reverse-video flag.
    #[serde(default)]
    pub reverse: bool,
}

/// Cell snapshot payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CellSnapshot {
    /// Columns.
    pub width: u16,
    /// Rows.
    pub height: u16,
    /// Row-major cells.
    pub cells: Vec<CellData>,
}

/// Comparison outcome with a ready-to-report diff.
pub struct CompareOutcome {
    /// Whether masked expected equals masked actual.
    pub equal: bool,
    /// Unified diff (empty when equal).
    pub diff: String,
}

/// Compile YAML `mask` entries into regexes.
pub fn compile_masks(entries: &[String]) -> Result<Vec<regex::Regex>> {
    entries
        .iter()
        .map(|entry| {
            if let Some(region) = entry.strip_prefix("region:") {
                return Err(SnapshotError::Message(format!(
                    "region masks (e.g. {region:?}) arrive in v2; use regex for now"
                )));
            }
            regex::Regex::new(entry)
                .map_err(|e| SnapshotError::Message(format!("bad mask regex {entry:?}: {e}")))
        })
        .collect()
}

/// Replace every masked span with [`MASK_REPLACEMENT`].
#[must_use]
pub fn apply_masks(text: &str, masks: &[regex::Regex]) -> String {
    let mut out = text.to_string();
    for mask in masks {
        out = mask.replace_all(&out, MASK_REPLACEMENT).into_owned();
    }
    out
}

/// Golden path shared by save and compare (size-scoped `.txt`).
pub fn text_golden_path(dir: &Path, name: &str, width: u16, height: u16) -> PathBuf {
    PathBuf::from(snapshot_file(&dir.to_string_lossy(), name, width, height)).with_extension("txt")
}

/// Write a text golden, creating parent directories.
pub fn save_text(dir: &Path, name: &str, width: u16, height: u16, text: &str) -> Result<PathBuf> {
    let path = text_golden_path(dir, name, width, height);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| SnapshotError::Io(format!("create {}: {e}", parent.display())))?;
    }
    // Text goldens share the size-scoped path with a `.txt` sibling marker.
    let path = path.with_extension("txt");
    std::fs::write(&path, text)
        .map_err(|e| SnapshotError::Io(format!("write {}: {e}", path.display())))?;
    Ok(path)
}

/// Load a text golden written by [`save_text`].
pub fn load_text(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .map_err(|e| SnapshotError::Io(format!("read {}: {e}", path.display())))
}

/// Compare live text against a golden with masks applied to both sides.
///
/// Line endings normalize (`\r\n` → `\n`) so goldens compare byte-identical
/// regardless of checkout `core.autocrlf` settings (see `.gitattributes`).
pub fn compare_text(expected: &str, actual: &str, masks: &[regex::Regex]) -> CompareOutcome {
    let expected = expected.replace("\r\n", "\n");
    let actual = actual.replace("\r\n", "\n");
    let masked_expected = apply_masks(&expected, masks);
    let masked_actual = apply_masks(&actual, masks);
    let equal = masked_expected == masked_actual;
    CompareOutcome {
        equal,
        diff: if equal {
            String::new()
        } else {
            unified_diff(&masked_expected, &masked_actual)
        },
    }
}

/// Serialize a cell snapshot to canonical JSON.
pub fn cells_to_json(snapshot: &CellSnapshot) -> Result<String> {
    serde_json::to_string_pretty(snapshot)
        .map_err(|e| SnapshotError::Message(format!("encode cells: {e}")))
}

/// Parse a cell snapshot back (round-trip + golden loading).
pub fn cells_from_json(text: &str) -> Result<CellSnapshot> {
    serde_json::from_str(text).map_err(|e| SnapshotError::Message(format!("decode cells: {e}")))
}
