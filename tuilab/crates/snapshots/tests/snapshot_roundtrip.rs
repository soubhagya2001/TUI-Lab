//! Snapshot round-trips: save/load/compare with masking (docs/06 §6.2–6.3).

use std::path::PathBuf;

use tui_lab_snapshots::{
    apply_masks, cells_from_json, cells_to_json, compare_cells, compare_sixels, compare_text,
    compile_masks, load_cells, load_sixels, save_cells, save_sixels, save_text, CellData,
    CellSnapshot,
};
use tui_lab_terminal::StyledCell;

fn scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tuilab-snap-{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn text_save_compare_passes() {
    let dir = scratch_dir("pass");
    let text = "Dashboard\nCPU: 20%\n";
    let path = save_text(&dir, "dashboard", 80, 24, text).expect("save");
    let back = std::fs::read_to_string(&path).expect("reload");
    let outcome = compare_text(&back, text, &[]);
    assert!(outcome.equal);
    assert!(outcome.diff.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn masked_clock_passes_despite_changed_time() {
    let masks = compile_masks(&["\\d{2}:\\d{2}:\\d{2}".to_string()]).expect("compile");
    let expected = "built at 10:00:00\nok\n";
    let actual = "built at 10:00:37\nok\n";
    assert!(!compare_text(expected, actual, &[]).equal);
    let outcome = compare_text(expected, actual, &masks);
    assert!(outcome.equal, "mask should hide the clock");
    assert!(apply_masks(expected, &masks).contains("<masked>"));
}

#[test]
fn unmasked_change_fails_with_diff() {
    let outcome = compare_text("a\nb\nc\n", "a\nB\nc\n", &[]);
    assert!(!outcome.equal);
    assert!(outcome.diff.contains("- b"));
    assert!(outcome.diff.contains("+ B"));
}

#[test]
fn region_masks_blank_exact_areas() {
    let masks = compile_masks(&["region:bottom:1".to_string()]).expect("compile");
    let expected = "stable title\nclock 10:00:00\n";
    let actual = "stable title\nclock 10:00:37\n";
    assert!(!compare_text(expected, actual, &[]).equal);
    let outcome = compare_text(expected, actual, &masks);
    assert!(outcome.equal, "bottom row masked out");
}

#[test]
fn rect_and_top_masks_compose_with_regex() {
    let masks = compile_masks(&[
        "region:rect:0,0,5,1".to_string(),
        "region:top:0".to_string(),
        "\\d+".to_string(),
    ])
    .expect("compile");
    // top:0 blanks nothing; rect blanks "hello"; regex blanks digits.
    let masked = apply_masks("hello 123\nworld\n", &masks);
    assert_eq!(masked, "      <masked>\nworld\n");
}

#[test]
fn unknown_region_names_stay_errors() {
    let err = compile_masks(&["region:middle_earth".to_string()]).expect_err("region");
    assert!(err.to_string().contains("region:rect"), "{err}");
    let err = compile_masks(&["region:rect:1,2".to_string()]).expect_err("arity");
    assert!(err.to_string().contains("rect:X,Y,W,H"), "{err}");
}

#[test]
fn bad_regex_mask_is_rejected() {
    assert!(compile_masks(&["([".to_string()]).is_err());
}

#[test]
fn styled_cells_map_losslessly_onto_goldens() {
    let cell = CellData::from(StyledCell {
        x: 4,
        y: 1,
        character: 'R',
        fg: "red".to_string(),
        bg: "black".to_string(),
        bold: true,
        underline: false,
        reverse: false,
    });
    assert_eq!(cell.x, 4);
    assert_eq!(cell.y, 1);
    assert_eq!(cell.char, "R");
    assert_eq!(cell.fg, "red");
    assert!(cell.bold);
    let snapshot = CellSnapshot {
        width: 80,
        height: 24,
        cells: vec![cell],
    };
    let back = cells_from_json(&cells_to_json(&snapshot).expect("encode")).expect("decode");
    assert_eq!(snapshot, back);
}

#[test]
fn cells_round_trip_through_json() {
    let snapshot = CellSnapshot {
        width: 80,
        height: 24,
        cells: vec![CellData {
            x: 0,
            y: 0,
            char: "D".to_string(),
            fg: "white".to_string(),
            bg: "black".to_string(),
            bold: true,
            underline: false,
            reverse: false,
        }],
    };
    let json = cells_to_json(&snapshot).expect("encode");
    let back = cells_from_json(&json).expect("decode");
    assert_eq!(snapshot, back);
}

fn cell(x: u16, y: u16, char: &str, fg: &str) -> CellData {
    CellData {
        x,
        y,
        char: char.to_string(),
        fg: fg.to_string(),
        bg: "black".to_string(),
        bold: false,
        underline: false,
        reverse: false,
    }
}

#[test]
fn styled_goldens_round_trip_size_scoped() {
    // D1: cells goldens live beside text goldens with size in the name.
    let dir = std::env::temp_dir().join(format!("tuilab-cells-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let snapshot = CellSnapshot {
        width: 80,
        height: 24,
        cells: vec![cell(0, 0, "A", "red")],
    };
    let path = save_cells(&dir, "dash", 80, 24, &snapshot).expect("save");
    assert!(
        path.ends_with("dash/80x24.cells.json")
            || path.to_string_lossy().contains("80x24.cells.json")
    );
    let back = load_cells(&path).expect("load");
    assert_eq!(snapshot, back);
    // A color-only change is a diff (same text, different pixels).
    let mut changed = snapshot.clone();
    changed.cells[0].fg = "blue".to_string();
    let outcome = compare_cells(&snapshot, &changed);
    assert!(!outcome.equal);
    assert!(outcome.diff.contains("(0,0)"), "{}", outcome.diff);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn sixels_compare_exact_normalized() {
    // D1: whitespace-insensitive, otherwise byte-exact — no tolerance.
    assert!(compare_sixels(&["a".to_string()], &["a".to_string()]).equal);
    let diff = compare_sixels(&["a".to_string()], &["a".to_string(), "b".to_string()]);
    assert!(!diff.equal);
    assert!(
        diff.diff.contains("1 image(s) != 2 image(s)"),
        "{}",
        diff.diff
    );
}

#[test]
fn sixel_goldens_round_trip() {
    let dir = std::env::temp_dir().join(format!("tuilab-sixel-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let payloads = vec![b"#0;2;0;0;0~~".to_vec()];
    let path = save_sixels(&dir, "logo", 80, 24, &payloads).expect("save");
    assert!(path.to_string_lossy().contains("80x24.sixel.json"));
    let back = load_sixels(&path).expect("load");
    assert_eq!(back, ["#0;2;0;0;0~~".to_string()]);
    let outcome = compare_sixels(&back, &["#0;2;0;0;0~~".to_string()]);
    assert!(outcome.equal);
    let _ = std::fs::remove_dir_all(&dir);
}
