//! Pure session helpers.

use crate::constants::DEFAULT_TIMEOUT_MS;

/// Normalize a caller-supplied timeout: zero means "use the default".
#[must_use]
pub fn clamp_timeout_ms(ms: u64) -> u64 {
    if ms == 0 {
        DEFAULT_TIMEOUT_MS
    } else {
        ms
    }
}

/// Format a session id from a counter (`sess_001`, …). Shared by every
/// frontend so ids look the same over CLI, MCP, and future transports.
#[must_use]
pub fn session_id(counter: u64) -> String {
    format!("sess_{counter:03}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_falls_back_to_default() {
        assert_eq!(clamp_timeout_ms(0), DEFAULT_TIMEOUT_MS);
    }

    #[test]
    fn nonzero_passes_through() {
        assert_eq!(clamp_timeout_ms(250), 250);
    }

    #[test]
    fn session_ids_are_zero_padded() {
        assert_eq!(session_id(7), "sess_007");
    }
}
