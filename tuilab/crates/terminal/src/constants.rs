//! Screen-model limits (docs/03 §3.2).

/// Maximum columns the adapter will materialize (guard against hostile resizes).
pub const MAX_WIDTH: u16 = 500;
/// Maximum rows the adapter will materialize.
pub const MAX_HEIGHT: u16 = 200;
/// Scrollback rows retained per session.
pub const SCROLLBACK_ROWS: usize = 1_000;
