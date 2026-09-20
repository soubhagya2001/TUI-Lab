//! Golden snapshot store: text + cell-JSON save/load/compare (docs/06 §6.2–6.3).
//!
//! Masking has two layers, applied in order: `region:` entries blank
//! rectangular areas to spaces first (positions stay stable, diffs stay
//! readable), then regex entries replace volatile spans. Nothing here
//! touches a PTY — callers feed screen dumps.

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

/// One compiled mask: regex span replacement or rectangular blanking.
#[derive(Debug, Clone)]
pub enum Mask {
    /// Replace every match with [`MASK_REPLACEMENT`].
    Regex(regex::Regex),
    /// Blank a screen area to spaces.
    Region(RegionSpec),
}

/// Rectangular screen area, 0-based columns/rows, clamped to the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionSpec {
    /// Left column.
    pub x: usize,
    /// Top row.
    pub y: usize,
    /// Width in columns.
    pub width: usize,
    /// Height in rows.
    pub height: usize,
}

/// Compile YAML `mask` entries: plain regexes plus `region:rect:X,Y,W,H`,
/// `region:top:N` (first N rows), `region:bottom:N` (last N rows).
pub fn compile_masks(entries: &[String]) -> Result<Vec<Mask>> {
    entries
        .iter()
        .map(|entry| {
            if let Some(spec) = entry.strip_prefix("region:") {
                return parse_region(spec, entry).map(Mask::Region);
            }
            regex::Regex::new(entry)
                .map(Mask::Regex)
                .map_err(|e| SnapshotError::Message(format!("bad mask regex {entry:?}: {e}")))
        })
        .collect()
}

/// Parse the body after `region:`.
fn parse_region(spec: &str, entry: &str) -> Result<RegionSpec> {
    let bad = || {
        SnapshotError::Message(format!(
            "bad region mask {entry:?}: want region:rect:X,Y,W,H, region:top:N, or region:bottom:N"
        ))
    };
    if let Some(rest) = spec.strip_prefix("rect:") {
        let nums: Option<Vec<usize>> = rest.split(',').map(|part| part.parse().ok()).collect();
        match nums.as_deref() {
            Some([x, y, width, height]) => Ok(RegionSpec {
                x: *x,
                y: *y,
                width: *width,
                height: *height,
            }),
            _ => Err(bad()),
        }
    } else if let Some(rest) = spec.strip_prefix("top:") {
        let rows: usize = rest.parse().map_err(|_| bad())?;
        Ok(RegionSpec {
            x: 0,
            y: 0,
            width: usize::MAX,
            height: rows,
        })
    } else if let Some(rest) = spec.strip_prefix("bottom:") {
        let rows: usize = rest.parse().map_err(|_| bad())?;
        Ok(RegionSpec {
            x: 0,
            y: usize::MAX,
            width: usize::MAX,
            height: rows,
        })
    } else {
        Err(bad())
    }
}

/// Replace every masked span with [`MASK_REPLACEMENT`]: regions blank first,
/// then regexes run over the blanked text.
#[must_use]
pub fn apply_masks(text: &str, masks: &[Mask]) -> String {
    let mut out = text.to_string();
    for mask in masks {
        match mask {
            Mask::Regex(pattern) => {
                out = pattern.replace_all(&out, MASK_REPLACEMENT).into_owned();
            }
            Mask::Region(spec) => out = apply_region(&out, spec),
        }
    }
    out
}

/// Blank a rectangular area to spaces. Short lines pad out to the region
/// edge so both sides of a comparison blank identically; rows outside the
/// text are ignored. Widths resolve against the widest line, so `top:N` and
/// `bottom:N` (encoded with infinite width) cover whole rows exactly.
fn apply_region(text: &str, spec: &RegionSpec) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let max_width = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    // `bottom:N` anchors to the last N rows once the height is known.
    let y = if spec.y == usize::MAX {
        lines.len().saturating_sub(spec.height)
    } else {
        spec.y
    };
    let end = spec.x.saturating_add(spec.width).min(max_width);
    let rendered: Vec<String> = lines
        .iter()
        .enumerate()
        .map(|(row, line)| {
            if row < y || row >= y.saturating_add(spec.height) || spec.x >= end {
                return line.to_string();
            }
            let mut chars: Vec<char> = line.chars().collect();
            while chars.len() < end {
                chars.push(' ');
            }
            for col in spec.x..end.min(chars.len()) {
                chars[col] = ' ';
            }
            chars.into_iter().collect()
        })
        .collect();
    let mut out = rendered.join("\n");
    if text.ends_with('\n') {
        out.push('\n');
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
pub fn compare_text(expected: &str, actual: &str, masks: &[Mask]) -> CompareOutcome {
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

impl From<tui_lab_terminal::StyledCell> for CellData {
    /// Map the grid-owned cell onto the golden format. Lossless by
    /// construction: every field has a counterpart.
    fn from(cell: tui_lab_terminal::StyledCell) -> Self {
        Self {
            x: cell.x as u16,
            y: cell.y as u16,
            char: cell.character.to_string(),
            fg: cell.fg,
            bg: cell.bg,
            bold: cell.bold,
            underline: cell.underline,
            reverse: cell.reverse,
        }
    }
}
