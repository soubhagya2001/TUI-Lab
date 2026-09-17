//! Test orchestration: sessions, lifecycle, step pipeline (docs/02).
//!
//! Phase 0 scaffold — types arrive in Phase 2.

pub mod constants;
pub mod context;
pub mod error;
pub mod result;
pub mod runner;
pub mod utils;

pub use context::TestContext;
pub use result::{FailureInfo, StepResult, SuiteResult, TerminalInfo};
pub use runner::{run_file, RunOptions};
