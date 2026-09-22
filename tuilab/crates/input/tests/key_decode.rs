//! Key-decoding matrix: every sequence decodes and round-trips.

use tui_lab_input::{decode_key, encode_key, Decode, Key};

fn key(bytes: &[u8]) -> (Key, usize) {
    match decode_key(bytes) {
        Decode::Key(key, len) => (key, len),
        other => panic!("expected key, got {other:?}"),
    }
}

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
        assert_eq!(key(bytes), (Key::Named(name), 3), "{name}");
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
        assert_eq!(key(bytes), (Key::Named(name), len), "{name}");
    }
}

#[test]
fn singles_modifiers_and_text() {
    assert_eq!(key(b"\r"), (Key::Named("ENTER"), 1));
    assert_eq!(key(b"\t"), (Key::Named("TAB"), 1));
    assert_eq!(key(b"q"), (Key::Char('q'), 1));
    assert_eq!(key(b"/"), (Key::Char('/'), 1));
    assert_eq!(key(b"\x03"), (Key::Ctrl('C'), 1));
    assert_eq!(key(b"\x1bx"), (Key::Alt('x'), 2));
    assert_eq!(key("é".as_bytes()), (Key::Char('é'), 2));
}

#[test]
fn incomplete_sequences_wait_for_more() {
    assert_eq!(decode_key(b""), Decode::Incomplete);
    assert_eq!(decode_key(b"\x1b"), Decode::Incomplete);
    assert_eq!(decode_key(b"\x1b["), Decode::Incomplete);
    assert_eq!(decode_key(b"\x1bO"), Decode::Incomplete);
    assert_eq!(decode_key(b"\x1b[2"), Decode::Incomplete);
    assert_eq!(decode_key(b"\x1b[?25"), Decode::Incomplete);
}

#[test]
fn unknown_sequences_skip_instead_of_poisoning_carry() {
    // R2: complete-but-unknown input is dropped (forwarded verbatim by the
    // caller), never accumulated.
    assert_eq!(decode_key(b"\x1b[X"), Decode::Skip(3));
    assert_eq!(decode_key(b"\x1b[?25h"), Decode::Skip(6));
    assert_eq!(decode_key(b"\x1b[999~"), Decode::Skip(6));
    assert_eq!(decode_key(b"\x1bOX"), Decode::Skip(3));
    // Absurdly long parameter runs are capped, not carried forever.
    let mut long = b"\x1b[".to_vec();
    long.extend([b'1'; 64]);
    assert_eq!(decode_key(&long), Decode::Skip(16));
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
        let (k, len) = key(sample);
        assert_eq!(len, sample.len());
        let name = match k {
            Key::Named(name) => name.to_string(),
            Key::Char(c) => c.to_string(),
            Key::Ctrl(c) => format!("CTRL+{c}"),
            Key::Alt(c) => format!("ALT+{c}"),
        };
        assert_eq!(encode_key(&name).expect("encodes").as_slice(), *sample);
    }
}

#[test]
fn fuzzed_bytes_never_grow_carry_without_bound() {
    // R2: deterministic pseudo-fuzz over byte soup, drained recorder-style.
    // Incomplete tails (partial ESC/UTF-8) are the only legal residue.
    let mut state: u32 = 0x1234_5678;
    let mut next_byte = || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        (state & 0xFF) as u8
    };
    let mut carry: Vec<u8> = Vec::new();
    for _ in 0..50_000 {
        carry.push(next_byte());
        let mut offset = 0;
        while offset < carry.len() {
            match decode_key(&carry[offset..]) {
                Decode::Incomplete => break,
                Decode::Key(_, len) | Decode::Skip(len) => offset += len,
            }
        }
        carry.drain(..offset);
        assert!(
            carry.len() < 32,
            "carry must stay bounded, got {}",
            carry.len()
        );
    }
}
