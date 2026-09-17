//! Runtime supervisor: timeouts, retries, polling, parallelism (docs/02 §2.2).
//!
//! Phase 0 scaffold — implementation arrives in Phase 2.

pub mod constants;
pub mod error;
pub mod supervisor;
pub mod utils;
pub mod wait;

pub use supervisor::run_with_timeout;
pub use wait::{matches, wait_for_text, WaitOutcome};
