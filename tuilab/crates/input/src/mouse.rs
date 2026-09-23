//! SGR mouse encoding: click/drag/scroll as terminals send them (docs/03 §3.3).
//!
//! SGR only (`ESC [ < Cb ; Cx ; Cy M/m`) — the modern path every supported
//! framework parses; legacy X10 is out of scope. Coordinates are 1-based
//! (top-left cell is 1,1); zeros clamp to 1 rather than erroring, because a
//! clamped click still tests the right code path while a panic tests nothing.

/// Mouse buttons for press events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    /// Left button (`Cb = 0`).
    Left,
    /// Middle button (`Cb = 1`).
    Middle,
    /// Right button (`Cb = 2`).
    Right,
}

impl MouseButton {
    fn code(self) -> u16 {
        match self {
            Self::Left => 0,
            Self::Middle => 1,
            Self::Right => 2,
        }
    }
}

/// One SGR event: `ESC [ < Cb ; Cx ; Cy M` (press) or `… m` (release).
fn sgr(code: u16, x: u16, y: u16, release: bool) -> Vec<u8> {
    let terminator = if release { 'm' } else { 'M' };
    format!("\x1b[<{code};{};{}{terminator}", x.max(1), y.max(1)).into_bytes()
}

/// Left/middle/right press at 1-based `(x, y)`.
pub fn mouse_press(button: MouseButton, x: u16, y: u16) -> Vec<u8> {
    sgr(button.code(), x, y, false)
}

/// Button release at 1-based `(x, y)`, button-coded (`Cb=<button>` + `m`).
///
/// Deliberately NOT the generic `Cb=3` release: ConPTY does not translate
/// `ESC[<3;x;ym` into a console input record (proven: the event never
/// arrives), while button-coded release works on ConPTY and parses on raw
/// PTYs (crossterm maps `Down` + `m` to `Up`). Documented in `docs/03`.
pub fn mouse_release(button: MouseButton, x: u16, y: u16) -> Vec<u8> {
    sgr(button.code(), x, y, true)
}

/// Wheel-up tick at 1-based `(x, y)` (`Cb = 64`).
pub fn mouse_scroll_up(x: u16, y: u16) -> Vec<u8> {
    sgr(64, x, y, false)
}

/// Wheel-down tick at 1-based `(x, y)` (`Cb = 65`).
pub fn mouse_scroll_down(x: u16, y: u16) -> Vec<u8> {
    sgr(64 + 1, x, y, false)
}

/// Hover (motion with no button down) at 1-based `(x, y)`.
///
/// Encoded as `Cb = 35` (`3` no-button + `32` motion flag) with `M`.
/// Terminals only report these with any-motion tracking (1003); plain
/// click tracking (1000) stays silent, so hover steps never appear
/// spuriously in recordings.
pub fn mouse_hover(x: u16, y: u16) -> Vec<u8> {
    sgr(35, x, y, false)
}

/// A decoded incoming mouse action (what the user's terminal sends while
/// recording with tracking enabled).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseAction {
    /// Button press.
    Press(MouseButton),
    /// Button-coded release.
    Release(MouseButton),
    /// Wheel up tick.
    ScrollUp,
    /// Wheel down tick.
    ScrollDown,
}

/// Decoded mouse input plus its 1-based cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseInput {
    /// What happened.
    pub action: MouseAction,
    /// 1-based column.
    pub x: u16,
    /// 1-based row.
    pub y: u16,
}

/// Decode `ESC [ < Cb ; Cx ; Cy M|m` at the front of `buf`.
///
/// Returns `(Some(event), len)` on success, `(None, 0)` when the buffer may
/// hold a partial event (wait for more bytes), and `(None, skip)` for junk
/// that must be dropped to keep recorder `carry` bounded. Motion events
/// (button held while moving) are skipped as noise — drags still record as
/// their press + release pair.
pub fn decode_mouse(buf: &[u8]) -> (Option<MouseInput>, usize) {
    const PREFIX: &[u8] = b"\x1b[<";
    const MAX_SCAN: usize = 32;
    if !buf.starts_with(PREFIX) {
        return (None, 0);
    }
    let scan_end = buf.len().min(MAX_SCAN);
    let Some(rel) = buf[PREFIX.len()..scan_end]
        .iter()
        .position(|&b| b == b'M' || b == b'm')
    else {
        // No terminator yet: partial event if the buffer is plausibly a
        // prefix, junk if it already overran the cap.
        return if buf.len() >= MAX_SCAN {
            (None, PREFIX.len())
        } else {
            (None, 0)
        };
    };
    let end = PREFIX.len() + rel;
    let release = buf[end] == b'm';
    let mut parts = buf[PREFIX.len()..end].split(|&b| b == b';');
    let (Some(cb), Some(x), Some(y), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return (None, end + 1);
    };
    let number = |digits: &[u8]| {
        std::str::from_utf8(digits)
            .ok()
            .and_then(|text| text.parse::<u16>().ok())
    };
    let (Some(cb), Some(x), Some(y)) = (number(cb), number(x), number(y)) else {
        return (None, end + 1);
    };
    // Motion flag: position noise between press and release — skip it.
    if !release && cb & 32 == 32 {
        return (None, end + 1);
    }
    let button = match cb & 0b11 {
        0 => MouseButton::Left,
        1 => MouseButton::Middle,
        2 => MouseButton::Right,
        // Generic Cb=3 release cannot be attributed — skip.
        _ => return (None, end + 1),
    };
    let action = match (cb, release) {
        (64, false) => MouseAction::ScrollUp,
        (65, false) => MouseAction::ScrollDown,
        (_, true) => MouseAction::Release(button),
        (_, false) => MouseAction::Press(button),
    };
    let input = MouseInput {
        action,
        x: x.max(1),
        y: y.max(1),
    };
    (Some(input), end + 1)
}

/// Drag byte shape: press-at-start + release-at-end, for format reference
/// and byte-level tests. Live gestures must still travel as separate writes
/// (see `encode_key` docs); this helper composes the pair.
pub fn mouse_drag(x1: u16, y1: u16, x2: u16, y2: u16) -> Vec<u8> {
    let mut bytes = mouse_press(MouseButton::Left, x1, y1);
    bytes.extend_from_slice(&mouse_release(MouseButton::Left, x2, y2));
    bytes
}
