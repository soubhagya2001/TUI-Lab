//! Snapshot round-trips: save/load/compare with masking (docs/06 §6.2–6.3).

use std::path::PathBuf;

use tui_lab_snapshots::{
    apply_masks, cells_from_json, cells_to_json, compare_text, compile_masks, save_text, CellData,
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
