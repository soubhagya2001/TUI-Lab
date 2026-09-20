//! Golden snapshots: text + cell-JSON store, diff, masking (docs/06 §6.2–6.3).
//!
//! Pure create/compare over caller-supplied dumps; no PTY in this crate.

pub mod constants;
pub mod error;
pub mod store;
pub mod utils;

pub use store::{
    apply_masks, cells_from_json, cells_to_json, compare_text, compile_masks, load_text, save_text,
    text_golden_path, CellData, CellSnapshot, CompareOutcome, Mask, RegionSpec, MASK_REPLACEMENT,
};
