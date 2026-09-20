//! Terminal emulator adapter over a reused grid crate (docs/03 §3.2).
//!
//! DECIDED: `alacritty_terminal` owns the grid/state machine; this crate
//! adapts it to screen text, styled cells, and DSR handshake forwarding.

pub mod constants;
pub mod emulator;
pub mod error;
pub mod utils;

pub use emulator::{Emulator, ForwardingListener, PtySink, StyledCell};
