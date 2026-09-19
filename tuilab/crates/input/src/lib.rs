//! Keyboard/mouse input encoding (docs/03 §3.3).
//!
//! Phase 0 scaffold — key tables and encoders arrive in Phase 1.

pub mod constants;
pub mod decode;
pub mod encoding;
pub mod error;
pub mod utils;

pub use decode::{decode_key, Key};
pub use encoding::{encode_key, encode_text};
