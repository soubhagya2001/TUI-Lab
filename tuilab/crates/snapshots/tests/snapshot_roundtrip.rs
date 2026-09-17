//! Snapshot round-trips: save/load/compare with masking (docs/06 §6.2–6.3).

use std::path::PathBuf;

use tui_lab_snapshots::{
    apply_masks, cells_from_json, cells_to_json, compare_text, compile_masks, save_text, CellData,
    CellSnapshot,
};

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
fn region_masks_are_rejected_with_guidance() {
    let err = compile_masks(&["region:bottom_status".to_string()]).expect_err("region mask");
    assert!(err.to_string().contains("v2"));
}

#[test]
fn bad_regex_mask_is_rejected() {
    assert!(compile_masks(&["([".to_string()]).is_err());
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
