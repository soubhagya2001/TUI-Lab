//! Key and text encoding to terminal bytes (docs/03 §3.3).
//!
//! Named keys follow xterm sequences; mouse actions follow SGR (see
//! [`crate::mouse`]). Unknown names are an error, not silent garbage.

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
    if let Some(result) = mouse(&normalized) {
        return result;
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

/// Mouse actions: `CLICK x y`, `RIGHT_CLICK x y`, `SCROLL_UP x y`,
/// `SCROLL_DOWN x y`, `RELEASE x y` (1-based cells). `None` means "not
/// a mouse name" so others fall through.
///
/// There is deliberately no single-blob `DRAG`: streaming parsers
/// (crossterm included) reject glued escape sequences, so gestures are
/// press + `RELEASE` as separate steps — exactly how real event streams
/// arrive. See `mouse_drag` for the byte shapes this composes.
fn mouse(normalized: &str) -> Option<Result<Vec<u8>>> {
    use crate::mouse::{
        mouse_press, mouse_release, mouse_scroll_down, mouse_scroll_up, MouseButton,
    };
    let mut parts = normalized.split_whitespace();
    let kind = parts.next()?;
    let rest: Vec<&str> = parts.collect();
    let mut coords = Vec::with_capacity(rest.len());
    for part in &rest {
        match part.parse::<u16>() {
            Ok(value) => coords.push(value),
            Err(_) => {
                return match kind {
                    "CLICK" | "RIGHT_CLICK" | "MIDDLE_CLICK" | "RELEASE" | "SCROLL_UP"
                    | "SCROLL_DOWN" | "DRAG" => Some(Err(InputError::UnknownKey(format!(
                        "{normalized}: coordinates must be integers"
                    )))),
                    _ => None,
                };
            }
        }
    }
    let at = |index: usize| coords.get(index).copied().unwrap_or(0);
    let shape = |want: &str| InputError::UnknownKey(format!("{normalized}: expected {want}"));
    let bytes = match (kind, coords.len()) {
        ("CLICK", 2) => mouse_press(MouseButton::Left, at(0), at(1)),
        ("RIGHT_CLICK", 2) => mouse_press(MouseButton::Right, at(0), at(1)),
        ("MIDDLE_CLICK", 2) => mouse_press(MouseButton::Middle, at(0), at(1)),
        // Left release: the overwhelmingly common gesture end. Button-coded
        // (not generic Cb=3) because ConPTY only translates button-coded
        // releases into input records (see mouse.rs).
        ("RELEASE", 2) => mouse_release(MouseButton::Left, at(0), at(1)),
        ("SCROLL_UP", 2) => mouse_scroll_up(at(0), at(1)),
        ("SCROLL_DOWN", 2) => mouse_scroll_down(at(0), at(1)),
        ("CLICK" | "RIGHT_CLICK" | "MIDDLE_CLICK" | "RELEASE" | "SCROLL_UP" | "SCROLL_DOWN", _) => {
            return Some(Err(shape("2 coordinates: e.g. CLICK 10 5")));
        }
        _ => return None,
    };
    Some(Ok(bytes))
}
