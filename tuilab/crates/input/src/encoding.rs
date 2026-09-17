//! Key and text encoding to terminal bytes (docs/03 §3.3).
//!
//! Named keys follow xterm sequences. Mouse sequences are explicitly
//! deferred to v2 (see docs/14 §14.4) — requesting one is an error, not
//! silent garbage.

use crate::constants::{
    KEY_BACKTAB, KEY_DELETE, KEY_DOWN, KEY_END, KEY_ENTER, KEY_ESCAPE, KEY_HOME, KEY_INSERT,
    KEY_LEFT, KEY_PAGE_DOWN, KEY_PAGE_UP, KEY_RIGHT, KEY_TAB, KEY_UP,
};
use crate::error::{InputError, Result};
use crate::utils::normalize_key;

/// Encode a `press` argument: a named key, `CTRL+x`, `ALT+x`, or a single
/// printable character. Matching is case-insensitive.
pub fn encode_key(name: &str) -> Result<Vec<u8>> {
    let normalized = normalize_key(name);

    if let Some(bytes) = named(&normalized) {
        return Ok(bytes.to_vec());
    }
    if let Some(rest) = normalized.strip_prefix("CTRL+") {
        return ctrl(rest);
    }
    if normalized.starts_with("ALT+") {
        // Preserve the original case (`ALT+x` != `ALT+X`).
        return alt(&name["ALT+".len()..]);
    }
    let mut chars = name.chars();
    match (chars.next(), chars.next()) {
        // Single literal character: preserve the original case (`q` != `Q`).
        (Some(c), None) => Ok(c.to_string().into_bytes()),
        _ => Err(InputError::UnknownKey(name.to_string())),
    }
}

/// Encode a `type` argument verbatim as UTF-8 (no Enter appended).
pub fn encode_text(text: &str) -> Vec<u8> {
    text.as_bytes().to_vec()
}

fn named(normalized: &str) -> Option<&'static [u8]> {
    Some(match normalized {
        KEY_ENTER => b"\r",
        KEY_ESCAPE => b"\x1b",
        KEY_TAB => b"\t",
        KEY_BACKTAB => b"\x1b[Z",
        KEY_UP => b"\x1b[A",
        KEY_DOWN => b"\x1b[B",
        KEY_RIGHT => b"\x1b[C",
        KEY_LEFT => b"\x1b[D",
        KEY_HOME => b"\x1b[H",
        KEY_END => b"\x1b[F",
        KEY_PAGE_UP => b"\x1b[5~",
        KEY_PAGE_DOWN => b"\x1b[6~",
        KEY_INSERT => b"\x1b[2~",
        KEY_DELETE => b"\x1b[3~",
        "F1" => b"\x1bOP",
        "F2" => b"\x1bOQ",
        "F3" => b"\x1bOR",
        "F4" => b"\x1bOS",
        "F5" => b"\x1b[15~",
        "F6" => b"\x1b[17~",
        "F7" => b"\x1b[18~",
        "F8" => b"\x1b[19~",
        "F9" => b"\x1b[20~",
        "F10" => b"\x1b[21~",
        "F11" => b"\x1b[23~",
        "F12" => b"\x1b[24~",
        _ => return None,
    })
}

/// `CTRL+A`..`CTRL+Z` and the `CTRL+[@[\]^_?]` range → control codes.
fn ctrl(rest: &str) -> Result<Vec<u8>> {
    let mut chars = rest.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_uppercase() => Ok(vec![c as u8 - b'A' + 1]),
        (Some(c), None) if "@[\\]^_?".contains(c) => Ok(vec![c as u8 - b'@']),
        _ => Err(InputError::UnknownKey(format!("CTRL+{rest}"))),
    }
}

/// `ALT+x` → ESC followed by the character bytes.
fn alt(rest: &str) -> Result<Vec<u8>> {
    let mut bytes = vec![0x1b];
    if rest.is_empty() {
        return Err(InputError::UnknownKey("ALT+".to_string()));
    }
    bytes.extend_from_slice(rest.as_bytes());
    Ok(bytes)
}
