//! Pure assertion helpers.

/// Substring check used by `text_visible` conditions.
#[must_use]
pub fn contains(haystack: &str, needle: &str) -> bool {
    haystack.contains(needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_visible_text() {
        assert!(contains("Welcome to Dashboard", "Dashboard"));
    }

    #[test]
    fn misses_absent_text() {
        assert!(!contains("Welcome", "Dashboard"));
    }
}
