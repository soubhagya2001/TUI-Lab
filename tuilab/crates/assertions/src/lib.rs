//! Assertion engine over terminal state (docs/06 §6.1): taxonomy,
//! JSON mapping, and evaluation against plain screen views.

pub mod conditions;
pub mod constants;
pub mod error;
pub mod utils;

pub use conditions::{condition_from_json, evaluate, Condition, ScreenView, Verdict};
