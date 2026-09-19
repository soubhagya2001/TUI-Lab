//! Key decoding: raw terminal bytes → key events (docs/03 §3.3).
//!
//! Inverse of [`crate::encoding`] for the recorder: parses complete key
//! sequences (arrows, function keys, CTRL/ALT chords, UTF-8 text) out of a
//! byte stream. Returns `None` when the buffer holds only an incomplete
//! sequence — the caller must feed more bytes, except a trailing lone ESC,
//! which the caller resolves with its own deadline (ESC key vs Alt prefix).

use crate::constants::{
    KEY_BACKTAB, KEY_DELETE, KEY_DOWN, KEY_END, KEY_HOME, KEY_INSERT, KEY_LEFT, KEY_PAGE_DOWN,
    KEY_PAGE_UP, KEY_RIGHT, KEY_TAB, KEY_UP,
};

/// A decoded keypress.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    /// Named key from the key table (`ENTER`, `DOWN`, `F5`, …).
    Named(&'static str),
    /// Printable character (coalesce runs into `type` steps).
    Char(char),
    /// `CTRL+X` (already uppercased).
    Ctrl(char),
    /// `ALT+x` (original case preserved).
    Alt(char),
}

/// Decode the first key in `buf`: `(key, bytes consumed)`.
///
/// `None` means incomplete — wait for more bytes (or apply the lone-ESC rule).
pub fn decode_key(buf: &[u8]) -> Option<(Key, usize)> {
    let &first = buf.first()?;
    match first {
        // Single-byte controls with dedicated names.
        0x0d | 0x0a => return Some((Key::Named("ENTER"), 1)),
        0x09 => return Some((Key::Named(KEY_TAB), 1)),
        0x7f => return Some((Key::Char('\u{7f}'), 1)),
        // Other C0 controls map to CTRL chords (0x1b handled below).
        0x00..=0x08 | 0x0b | 0x0c | 0x0e..=0x1a => {
            let upper = (first + b'A' - 1) as char;
            return Some((Key::Ctrl(upper), 1));
        }
        0x1c..=0x1f => {
            let glyph = (first - 0x1c + b'\\') as char;
            return Some((Key::Ctrl(glyph), 1));
        }
        0x1b => {} // Escape sequences below.
        0x20..=0x7e => {
            return Some((Key::Char(first as char), 1));
        }
        _ => {} // Possibly multi-byte UTF-8; handled after ASCII fast paths.
    }

    // Multi-byte sequences starting with ESC.
    if first == 0x1b {
        if buf.len() < 2 {
            return None;
        }
        match buf[1] {
            b'[' => return decode_csi(buf),
            b'O' => {
                if buf.len() < 3 {
                    return None;
                }
                let named = match buf[2] {
                    b'P' => "F1",
                    b'Q' => "F2",
                    b'R' => "F3",
                    b'S' => "F4",
                    _ => return None,
                };
                return Some((Key::Named(named), 3));
            }
            // ESC immediately followed by another byte: Alt chord (v1 rule —
            // interactive terminals send Alt this way; see recorder docs).
            _ => {
                let tail = &buf[1..];
                let text = std::str::from_utf8(tail).ok()?;
                let c = text.chars().next()?;
                return Some((Key::Alt(c), 1 + c.len_utf8()));
            }
        }
    }

    // Non-ASCII: decode one UTF-8 char (or wait for the rest).
    let text = std::str::from_utf8(buf).ok()?;
    let c = text.chars().next()?;
    Some((Key::Char(c), c.len_utf8()))
}

/// Decode `ESC [ …` sequences.
fn decode_csi(buf: &[u8]) -> Option<(Key, usize)> {
    if buf.len() < 3 {
        return None;
    }
    match buf[2] {
        b'A' => Some((Key::Named(KEY_UP), 3)),
        b'B' => Some((Key::Named(KEY_DOWN), 3)),
        b'C' => Some((Key::Named(KEY_RIGHT), 3)),
        b'D' => Some((Key::Named(KEY_LEFT), 3)),
        b'H' => Some((Key::Named(KEY_HOME), 3)),
        b'F' => Some((Key::Named(KEY_END), 3)),
        b'Z' => Some((Key::Named(KEY_BACKTAB), 3)),
        b'0'..=b'9' => {
            // `ESC [ <n> ~` — find the terminator.
            let end = buf.iter().position(|&b| b == b'~')?;
            let num: String = buf[2..end].iter().map(|&b| b as char).collect();
            let named = match num.as_str() {
                "2" => KEY_INSERT,
                "3" => KEY_DELETE,
                "5" => KEY_PAGE_UP,
                "6" => KEY_PAGE_DOWN,
                "15" => "F5",
                "17" => "F6",
                "18" => "F7",
                "19" => "F8",
                "20" => "F9",
                "21" => "F10",
                "23" => "F11",
                "24" => "F12",
                _ => return None,
            };
            Some((Key::Named(named), end + 1))
        }
        _ => None,
    }
}
