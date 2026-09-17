//! Full key-encoding matrix: every named key, modifiers, literals.

use tui_lab_input::{encode_key, encode_text};

#[test]
fn named_keys_encode_to_xterm_sequences() {
    let cases = [
        ("ENTER", b"\r".as_slice()),
        ("esc", b"\x1b".as_slice()),
        ("TAB", b"\t".as_slice()),
        ("BACKTAB", b"\x1b[Z".as_slice()),
        ("UP", b"\x1b[A".as_slice()),
        ("down", b"\x1b[B".as_slice()),
        ("RIGHT", b"\x1b[C".as_slice()),
        ("LEFT", b"\x1b[D".as_slice()),
        ("HOME", b"\x1b[H".as_slice()),
        ("END", b"\x1b[F".as_slice()),
        ("PGUP", b"\x1b[5~".as_slice()),
        ("PGDN", b"\x1b[6~".as_slice()),
        ("INSERT", b"\x1b[2~".as_slice()),
        ("DELETE", b"\x1b[3~".as_slice()),
        ("F1", b"\x1bOP".as_slice()),
        ("F4", b"\x1bOS".as_slice()),
        ("F5", b"\x1b[15~".as_slice()),
        ("F12", b"\x1b[24~".as_slice()),
    ];
    for (name, expected) in cases {
        assert_eq!(encode_key(name).expect(name).as_slice(), expected, "{name}");
    }
}

#[test]
fn control_keys_map_to_control_codes() {
    assert_eq!(encode_key("CTRL+C").expect("ctrl-c"), vec![0x03]);
    assert_eq!(encode_key("ctrl+d").expect("ctrl-d"), vec![0x04]);
    assert_eq!(encode_key("CTRL+Z").expect("ctrl-z"), vec![0x1A]);
}

#[test]
fn alt_keys_prefix_escape() {
    assert_eq!(encode_key("ALT+x").expect("alt-x"), b"\x1bx".to_vec());
}

#[test]
fn literal_characters_preserve_case() {
    assert_eq!(encode_key("q").expect("q"), b"q".to_vec());
    assert_eq!(encode_key("Q").expect("Q"), b"Q".to_vec());
    assert_eq!(encode_key("/").expect("slash"), b"/".to_vec());
}

#[test]
fn unknown_keys_are_errors_not_garbage() {
    assert!(encode_key("F13").is_err());
    assert!(encode_key("CTRL+ENTER").is_err());
    assert!(encode_key("").is_err());
}

#[test]
fn text_encodes_verbatim_utf8() {
    assert_eq!(encode_text("table"), b"table".to_vec());
    assert_eq!(encode_text("héllo").as_slice(), "héllo".as_bytes());
}
