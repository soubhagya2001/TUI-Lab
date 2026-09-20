//! Reporters: JSON results, JUnit XML, self-contained HTML — docs/11.
//!
//! Pure formatters over core result types; every format re-renders the
//! same stored JSON so outputs can never disagree.

pub mod constants;
pub mod error;
pub mod html;
pub mod json;
pub mod junit;
pub mod utils;

pub use html::to_html;
pub use json::{load_json, load_json_all, to_json, write_json, write_json_all};
pub use junit::{to_junit, to_junit_all};
