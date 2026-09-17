//! Pure input helpers.

/// Normalize a key name for case-insensitive matching (`down` -> `DOWN`).
#[must_use]
pub fn normalize_key(name: &str) -> String {
    name.to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowercases_become_canonical() {
        assert_eq!(normalize_key("down"), "DOWN");
    }

    #[test]
    fn canonical_names_are_stable() {
        assert_eq!(normalize_key("ENTER"), "ENTER");
    }
}
