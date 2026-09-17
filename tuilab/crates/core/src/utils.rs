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
}
