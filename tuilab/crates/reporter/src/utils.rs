//! Pure reporter helpers.

use crate::constants::MAX_RAW_BYTES;

/// Cap raw terminal bytes retained per failure bundle.
#[must_use]
pub fn truncate_bytes(data: &[u8]) -> &[u8] {
    &data[..data.len().min(MAX_RAW_BYTES)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_output_passes_through() {
        assert_eq!(truncate_bytes(b"hi"), b"hi");
    }

    #[test]
    fn long_output_is_capped() {
        let data = vec![b'x'; MAX_RAW_BYTES + 1];
        assert_eq!(truncate_bytes(&data).len(), MAX_RAW_BYTES);
    }
}
