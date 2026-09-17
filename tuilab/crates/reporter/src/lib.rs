//! Reporters: JSON results, JUnit XML (v1), HTML (v2) — docs/11.
//!
//! Phase 0 scaffold — implementation arrives in Phase 3.

pub mod constants;
pub mod error;
pub mod json;
pub mod junit;
pub mod utils;

pub use json::{load_json, load_json_all, to_json, write_json, write_json_all};
pub use junit::{to_junit, to_junit_all};
