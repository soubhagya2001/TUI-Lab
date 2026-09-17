//! PTY abstraction over Unix PTY and Windows ConPTY (docs/03, docs/12).
//!
//! OS branching lives only in this crate. Phase 0 scaffold —
//! `portable-pty` wiring arrives in Phase 1.

pub mod constants;
pub mod error;
pub mod session;
pub mod utils;

pub use session::{PtySession, SharedWriter, SpawnOptions};
