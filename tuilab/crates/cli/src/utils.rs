//! CLI helpers.

use crate::constants::{
    EXIT_CONFIG_ERROR, EXIT_OK, EXIT_PTY_ERROR, EXIT_TESTS_FAILED, EXIT_TIMEOUT,
};

/// Human-readable meaning of a CLI exit code (docs/07 §7.3).
#[must_use]
pub fn exit_message(code: i32) -> &'static str {
    match code {
        EXIT_OK => "ok: all tests passed",
        EXIT_TESTS_FAILED => "fail: at least one test failed",
        EXIT_CONFIG_ERROR => "error: config or schema problem",
        EXIT_PTY_ERROR => "error: app launch or PTY failure",
        EXIT_TIMEOUT => "error: timeout, hung process killed",
        _ => "error: unknown exit code",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_codes_have_messages() {
        assert!(exit_message(EXIT_OK).starts_with("ok"));
        assert!(exit_message(EXIT_TIMEOUT).starts_with("error"));
    }
}
