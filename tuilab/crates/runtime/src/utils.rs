//! Pure supervisor helpers.

use crate::constants::POLL_MS;

/// Linear backoff between poll attempts (attempt is 0-based).
#[must_use]
pub fn backoff_ms(attempt: u32) -> u64 {
    POLL_MS.saturating_mul(u64::from(attempt).saturating_add(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_attempt_waits_one_poll() {
        assert_eq!(backoff_ms(0), POLL_MS);
    }

    #[test]
    fn backoff_grows_linearly() {
        assert_eq!(backoff_ms(3), POLL_MS * 4);
    }
}
