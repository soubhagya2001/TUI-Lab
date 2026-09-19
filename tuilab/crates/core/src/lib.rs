//! Test orchestration: sessions, lifecycle, step pipeline (docs/02).
//!
//! Phase 0 scaffold — types arrive in Phase 2.

pub mod constants;
pub mod context;
pub mod error;
pub mod parallel;
pub mod result;
pub mod runner;
pub mod sessions;
pub mod utils;

pub use context::TestContext;
pub use parallel::run_suites;
pub use result::{FailureInfo, StepResult, SuiteResult, TerminalInfo};
pub use runner::{run_file, RunOptions, StepHook};
pub use sessions::{ClosedSession, LiveSession, NewSession, SessionRegistry};
