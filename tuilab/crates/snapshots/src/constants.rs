//! Snapshot store conventions (docs/06 §6.2).

/// Directory (relative to project root) holding golden snapshots.
pub const SNAPSHOTS_DIR: &str = "tests/snapshots";
/// Suffix for freshly captured snapshots awaiting approval.
pub const NEW_SUFFIX: &str = ".new";
/// Suffix for actual-output dumps on mismatch.
pub const ACTUAL_SUFFIX: &str = ".actual";
