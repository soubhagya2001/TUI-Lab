//! Session-based JSON test protocol v1 (docs/04).
//!
//! Phase 0 scaffold — serde types arrive in Phase 2.

pub mod actions;
pub mod constants;
pub mod error;
pub mod steps;
pub mod utils;

pub use actions::Action;
pub use steps::{
    Application, RegionAssertion, ResizeTo, SleepFor, SnapshotTake, Step, SuiteAssertion,
    TerminalConfig, TestFile, TextAssertion, WaitForExit, WaitForText,
};
