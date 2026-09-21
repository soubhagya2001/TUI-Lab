//! Pure PTY helpers.

use crate::constants::{BLOCKED_ENV_EXACT, BLOCKED_ENV_PREFIXES, DEFAULT_TERM};

/// Resolve the `$TERM` advertised to the child process.
#[must_use]
pub fn term_for(env_term: Option<&str>) -> &str {
    env_term.unwrap_or(DEFAULT_TERM)
}

/// Whether an env key may be passed to the child (S4). Blocks loader-hijack
/// and toolchain-poisoning keys; everything else (including `TERM`) passes.
#[must_use]
pub fn env_allowed(key: &str) -> bool {
    let upper = key.to_ascii_uppercase();
    !BLOCKED_ENV_EXACT.iter().any(|blocked| upper == *blocked)
        && !BLOCKED_ENV_PREFIXES
            .iter()
            .any(|prefix| upper.starts_with(prefix))
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

    #[test]
    fn blocks_loader_and_toolchain_keys() {
        for blocked in [
            "LD_PRELOAD",
            "Ld_Preload",
            "DYLD_INSERT_LIBRARIES",
            "CARGO_HOME",
            "CARGO_NET_OFFLINE",
            "PATH",
            "Path",
            "path",
        ] {
            assert!(!env_allowed(blocked), "{blocked} must be blocked");
        }
    }

    #[test]
    fn allows_ordinary_keys() {
        for allowed in ["TERM", "RUST_LOG", "MYAPP_MODE", "LD", "CARGO", "PATHWAY"] {
            assert!(env_allowed(allowed), "{allowed} must pass");
        }
    }
}
