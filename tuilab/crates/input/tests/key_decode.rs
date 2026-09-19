//! Key-decoding matrix: every sequence decodes and round-trips.

use tui_lab_input::{decode_key, encode_key, Key};

#[test]
fn arrows_home_end_backtab() {
    let cases = [
        (b"\x1b[A".as_slice(), "UP"),
        (b"\x1b[B".as_slice(), "DOWN"),
        (b"\x1b[C".as_slice(), "RIGHT"),
        (b"\x1b[D".as_slice(), "LEFT"),
        (b"\x1b[H".as_slice(), "HOME"),
        (b"\x1b[F".as_slice(), "END"),
        (b"\x1b[Z".as_slice(), "BACKTAB"),
    ];
    for (bytes, name) in cases {
        assert_eq!(decode_key(bytes), Some((Key::Named(name), 3)), "{name}");
    }
}

#[test]
fn tilde_and_ss3_function_keys() {
    let cases = [
        (b"\x1b[2~".as_slice(), "INSERT", 4),
        (b"\x1b[3~".as_slice(), "DELETE", 4),
        (b"\x1b[5~".as_slice(), "PGUP", 4),
        (b"\x1bOP".as_slice(), "F1", 3),
        (b"\x1b[15~".as_slice(), "F5", 5),
        (b"\x1b[24~".as_slice(), "F12", 5),
    ];
    for (bytes, name, len) in cases {
        assert_eq!(decode_key(bytes), Some((Key::Named(name), len)), "{name}");
    }
}

#[test]
fn singles_modifiers_and_text() {
    assert_eq!(decode_key(b"\r"), Some((Key::Named("ENTER"), 1)));
    assert_eq!(decode_key(b"\t"), Some((Key::Named("TAB"), 1)));
    assert_eq!(decode_key(b"q"), Some((Key::Char('q'), 1)));
    assert_eq!(decode_key(b"/"), Some((Key::Char('/'), 1)));
    assert_eq!(decode_key(b"\x03"), Some((Key::Ctrl('C'), 1)));
    assert_eq!(decode_key(b"\x1bx"), Some((Key::Alt('x'), 2)));
    assert_eq!(decode_key("é".as_bytes()), Some((Key::Char('é'), 2)));
}

#[test]
fn incomplete_sequences_wait_for_more() {
    assert_eq!(decode_key(b""), None);
    assert_eq!(decode_key(b"\x1b"), None);
    assert_eq!(decode_key(b"\x1b["), None);
    assert_eq!(decode_key(b"\x1bO"), None);
    assert_eq!(decode_key(b"\x1b[2"), None);
}

#[test]
fn decoded_keys_reencode_to_the_same_bytes() {
    // Every decoded key must survive a decode→encode round-trip, otherwise
    // recorded suites replay different bytes than the user typed.
    let samples: &[&[u8]] = &[
        b"\x1b[B",
        b"\r",
        b"q",
        b"/",
        b"\x03",
        b"\x1b[15~",
        b"\x1bOP",
        b"\x1b[Z",
    ];
    for sample in samples {
        let (key, len) = decode_key(sample).expect("decodes");
        assert_eq!(len, sample.len());
        let name = match key {
            Key::Named(name) => name.to_string(),
            Key::Char(c) => c.to_string(),
            Key::Ctrl(c) => format!("CTRL+{c}"),
            Key::Alt(c) => format!("ALT+{c}"),
        };
        assert_eq!(encode_key(&name).expect("encodes").as_slice(), *sample);
    }
}
