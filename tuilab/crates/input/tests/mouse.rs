//! SGR mouse bytes: exact sequences, clamping, arity errors.

use tui_lab_input::{
    encode_key, mouse_drag, mouse_press, mouse_release, mouse_scroll_down, mouse_scroll_up,
    MouseButton,
};

#[test]
fn press_release_use_sgr_terminators() {
    assert_eq!(mouse_press(MouseButton::Left, 10, 5), b"\x1b[<0;10;5M");
    // Button-coded release (Cb=0+m): the form ConPTY actually translates.
    // Generic Cb=3 releases never arrive (proven live, Phase 9b).
    assert_eq!(mouse_release(MouseButton::Left, 10, 5), b"\x1b[<0;10;5m");
    assert_eq!(mouse_press(MouseButton::Right, 1, 1), b"\x1b[<2;1;1M");
}

#[test]
fn scroll_uses_wheel_codes() {
    assert_eq!(mouse_scroll_up(10, 5), b"\x1b[<64;10;5M");
    assert_eq!(mouse_scroll_down(10, 5), b"\x1b[<65;10;5M");
}

#[test]
fn drag_is_press_plus_release() {
    let mut expected = mouse_press(MouseButton::Left, 10, 5).to_vec();
    expected.extend_from_slice(&mouse_release(MouseButton::Left, 20, 5));
    assert_eq!(mouse_drag(10, 5, 20, 5), expected);
}

#[test]
fn zero_coords_clamp_to_one() {
    assert_eq!(mouse_press(MouseButton::Left, 0, 0), b"\x1b[<0;1;1M");
}

#[test]
fn key_names_encode_with_case_insensitivity() {
    assert_eq!(encode_key("CLICK 10 5").expect("click"), b"\x1b[<0;10;5M");
    assert_eq!(
        encode_key("click 10 5").expect("lowercase"),
        b"\x1b[<0;10;5M"
    );
    assert_eq!(
        encode_key("SCROLL_DOWN 3 7").expect("scroll"),
        b"\x1b[<65;3;7M"
    );
}

#[test]
fn drag_has_no_single_key_name() {
    // Gestures travel as separate writes (press, then RELEASE); a glued
    // blob is rejected by streaming parsers, so no such name exists.
    assert!(encode_key("DRAG 10 5 20 5").is_err());
}

#[test]
fn bad_arity_names_the_shape() {
    let err = encode_key("CLICK 10").expect_err("arity");
    assert!(err.to_string().contains("2 coordinates"), "{err}");
    let err = encode_key("CLICK x y").expect_err("integers");
    assert!(err.to_string().contains("integers"), "{err}");
}
