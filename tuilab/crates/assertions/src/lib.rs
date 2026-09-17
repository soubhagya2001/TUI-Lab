//! Assertion engine over terminal state (docs/06 §6.1).
//!
//! Phase 0 scaffold — taxonomy implementation arrives in Phase 2.

pub mod conditions;
pub mod constants;
pub mod error;
pub mod utils;

pub use conditions::{condition_from_json, evaluate, Condition, ScreenView, Verdict};
