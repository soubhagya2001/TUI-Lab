//! Pure PTY helpers.

use crate::constants::DEFAULT_TERM;

/// Resolve the `$TERM` advertised to the child process.
#[must_use]
pub fn term_for(env_term: Option<&str>) -> &str {
    env_term.unwrap_or(DEFAULT_TERM)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_term_when_unset() {
        assert_eq!(term_for(None), DEFAULT_TERM);
    }

    #[test]
    fn honors_override() {
        assert_eq!(term_for(Some("xterm")), "xterm");
    }
}
