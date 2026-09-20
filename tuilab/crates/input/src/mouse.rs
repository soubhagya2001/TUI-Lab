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
    sgr(65, x, y, false)
}

/// Drag byte shape: press-at-start + release-at-end, for format reference
/// and byte-level tests. Live gestures must still travel as separate writes
/// (see `encode_key` docs); this helper composes the pair.
pub fn mouse_drag(x1: u16, y1: u16, x2: u16, y2: u16) -> Vec<u8> {
    let mut bytes = mouse_press(MouseButton::Left, x1, y1);
    bytes.extend_from_slice(&mouse_release(MouseButton::Left, x2, y2));
    bytes
}
