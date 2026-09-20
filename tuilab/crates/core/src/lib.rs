//! Test orchestration: sessions, lifecycle, step pipeline (docs/02).
//!
//! Single-session runner, parallel fan-out, and the shared live-session
//! registry behind every frontend.

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
