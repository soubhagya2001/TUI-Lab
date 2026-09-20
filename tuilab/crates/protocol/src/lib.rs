//! Session-based JSON test protocol v1 (docs/04): actions, YAML steps,
//! strict validation. The single definition every frontend speaks.

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
