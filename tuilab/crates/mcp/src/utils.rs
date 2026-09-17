//! MCP helpers.

/// Format a session id from a counter.
#[must_use]
pub fn session_id(counter: u64) -> String {
    format!("sess_{counter:03}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_zero_padded_ids() {
        assert_eq!(session_id(7), "sess_007");
    }
}
