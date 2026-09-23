//! Terminal emulator adapter over a reused grid crate (docs/03 §3.2).
//!
//! DECIDED: `alacritty_terminal` owns the grid/state machine; this crate
//! adapts it to screen text, styled cells, and DSR handshake forwarding.

pub mod a11y;
pub mod constants;
pub mod emulator;
pub mod error;
pub mod utils;

pub use a11y::{build_tree, dump_tree, find_role, A11yNode, Role};
pub use emulator::{Emulator, ForwardingListener, PtySink, StyledCell};
