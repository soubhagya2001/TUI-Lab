//! Golden snapshots: text + cell-JSON store, diff, masking (docs/06 §6.2–6.3).
//!
//! Phase 0 scaffold — implementation arrives in Phase 2.

pub mod constants;
pub mod error;
pub mod store;
pub mod utils;

pub use store::{
    apply_masks, cells_from_json, cells_to_json, compare_text, compile_masks, load_text, save_text,
    CellData, CellSnapshot, CompareOutcome, MASK_REPLACEMENT,
};
