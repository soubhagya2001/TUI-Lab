//! Terminal emulator adapter over a reused grid crate (docs/03 §3.2).
//!
//! DECIDED: `alacritty_terminal` (or equivalent) owns the grid/state machine;
//! this crate adapts it to TUI Lab's screen model. Wiring arrives in Phase 1.

pub mod constants;
pub mod emulator;
pub mod error;
pub mod utils;

pub use emulator::{Emulator, ForwardingListener, PtySink};
