//! Pure protocol helpers.

use crate::constants::SCHEMA_ID;

/// Check whether a test file declares a schema this engine supports.
#[must_use]
pub fn is_supported_schema(id: &str) -> bool {
    id == SCHEMA_ID
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_v1_schema() {
        assert!(is_supported_schema("tui-lab/v1"));
    }

    #[test]
    fn rejects_unknown_schema() {
        assert!(!is_supported_schema("tui-lab/v9"));
    }
}
