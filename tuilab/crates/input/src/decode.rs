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

/// Decode outcome.
///
/// R2: `None` used to mean both "need more bytes" and "unknown junk", so
/// the recorder accumulated unknown sequences in `carry` forever. The two
/// cases are now distinct — junk is skipped (forwarded verbatim, no step),
/// only genuinely partial input waits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decode {
    /// A key plus bytes consumed.
    Key(Key, usize),
    /// Buffer holds only a partial sequence — feed more bytes.
    Incomplete,
    /// Invalid sequence — drop this many bytes (already forwarded).
    Skip(usize),
}

/// Decode the first key in `buf`.
pub fn decode_key(buf: &[u8]) -> Decode {
    let Some(&first) = buf.first() else {
        return Decode::Incomplete;
    };
    match first {
        // Single-byte controls with dedicated names.
        0x0d | 0x0a => return Decode::Key(Key::Named("ENTER"), 1),
        0x09 => return Decode::Key(Key::Named(KEY_TAB), 1),
        0x7f => return Decode::Key(Key::Char('\u{7f}'), 1),
        // Other C0 controls map to CTRL chords (0x1b handled below).
        0x00..=0x08 | 0x0b | 0x0c | 0x0e..=0x1a => {
            let upper = (first + b'A' - 1) as char;
            return Decode::Key(Key::Ctrl(upper), 1);
        }
        0x1c..=0x1f => {
            let glyph = (first - 0x1c + b'\\') as char;
            return Decode::Key(Key::Ctrl(glyph), 1);
        }
        0x1b => {} // Escape sequences below.
        0x20..=0x7e => {
            return Decode::Key(Key::Char(first as char), 1);
        }
        _ => {} // Possibly multi-byte UTF-8; handled after ASCII fast paths.
    }

    // Multi-byte sequences starting with ESC.
    if first == 0x1b {
        if buf.len() < 2 {
            return Decode::Incomplete;
        }
        match buf[1] {
            b'[' => return decode_csi(buf),
            b'O' => {
                if buf.len() < 3 {
                    return Decode::Incomplete;
                }
                let named = match buf[2] {
                    b'P' => "F1",
                    b'Q' => "F2",
                    b'R' => "F3",
                    b'S' => "F4",
                    // Complete but unknown SS3 — skip, don't poison carry.
                    _ => return Decode::Skip(3),
                };
                return Decode::Key(Key::Named(named), 3);
            }
            // ESC immediately followed by another byte: Alt chord (v1 rule —
            // interactive terminals send Alt this way; see recorder docs).
            _ => {
                return match std::str::from_utf8(&buf[1..]) {
                    Ok(text) => match text.chars().next() {
                        Some(c) => Decode::Key(Key::Alt(c), 1 + c.len_utf8()),
                        None => Decode::Incomplete,
                    },
                    // Invalid bytes are junk (skip); a `None` length means
                    // a partial char at the end (wait for more).
                    Err(e) => match e.error_len() {
                        Some(len) => Decode::Skip(1 + len),
                        None => Decode::Incomplete,
                    },
                };
            }
        }
    }

    // Non-ASCII: decode one UTF-8 char, skip invalid bytes, or wait for
    // the rest of a partial char.
    match std::str::from_utf8(buf) {
        Ok(text) => match text.chars().next() {
            Some(c) => Decode::Key(Key::Char(c), c.len_utf8()),
            None => Decode::Incomplete,
        },
        Err(e) => match e.error_len() {
            Some(len) => Decode::Skip(len),
            None => Decode::Incomplete,
        },
    }
}

/// Upper bound on CSI parameter bytes scanned for a final byte (R2).
///
/// Real key sequences terminate within a handful of bytes; beyond this the
/// buffer is junk, so skip instead of accumulating `carry` without end.
const MAX_CSI_SCAN: usize = 16;

/// Decode `ESC [ …` sequences.
fn decode_csi(buf: &[u8]) -> Decode {
    if buf.len() < 3 {
        return Decode::Incomplete;
    }
    match buf[2] {
        b'A' => Decode::Key(Key::Named(KEY_UP), 3),
        b'B' => Decode::Key(Key::Named(KEY_DOWN), 3),
        b'C' => Decode::Key(Key::Named(KEY_RIGHT), 3),
        b'D' => Decode::Key(Key::Named(KEY_LEFT), 3),
        b'H' => Decode::Key(Key::Named(KEY_HOME), 3),
        b'F' => Decode::Key(Key::Named(KEY_END), 3),
        b'Z' => Decode::Key(Key::Named(KEY_BACKTAB), 3),
        _ => decode_csi_params(buf),
    }
}

/// Decode a CSI sequence with parameter bytes (`ESC [ <params> <final>`).
///
/// A present-but-unknown final byte means complete junk: skip through it.
/// Parameter bytes with no final byte yet mean incomplete input — unless the
/// run exceeds [`MAX_CSI_SCAN`], in which case skip to keep `carry` bounded.
fn decode_csi_params(buf: &[u8]) -> Decode {
    let scan_end = buf.len().min(2 + MAX_CSI_SCAN);
    match buf[2..scan_end]
        .iter()
        .position(|&b| (0x40..=0x7e).contains(&b))
    {
        Some(rel) => {
            let end = 2 + rel;
            if buf[end] != b'~' {
                // Complete CSI, unknown purpose (e.g. `ESC[?25h`).
                return Decode::Skip(end + 1);
            }
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
                // Complete `~` sequence, unknown number.
                _ => return Decode::Skip(end + 1),
            };
            Decode::Key(Key::Named(named), end + 1)
        }
        None => {
            if buf.len() > 2 + MAX_CSI_SCAN {
                Decode::Skip(MAX_CSI_SCAN)
            } else {
                Decode::Incomplete
            }
        }
    }
}
