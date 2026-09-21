//! Pure reporter helpers.

use crate::constants::MAX_RAW_BYTES;

/// Cap raw terminal bytes retained per failure bundle.
#[must_use]
pub fn truncate_bytes(data: &[u8]) -> &[u8] {
    &data[..data.len().min(MAX_RAW_BYTES)]
}

/// Strip characters illegal in XML 1.0 and invisible in HTML (S5): C0
/// controls except tab/LF/CR, plus DEL. Terminal screens carry these
/// routinely (cursor addressing, NUL padding); emitting them raw breaks XML
/// parsers and muddies HTML. Applied inside both `escape()` fns so every
/// sink is covered.
#[must_use]
pub fn sanitize(text: &str) -> String {
    text.chars()
        .filter(|c| {
            !matches!(
                c,
                '\u{00}'..='\u{08}' | '\u{0B}' | '\u{0C}' | '\u{0E}'..='\u{1F}' | '\u{7F}'
            )
        })
        .collect()
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

    #[test]
    fn sanitize_strips_illegal_xml_chars() {
        assert_eq!(sanitize("a\x00b\x07c\x1bd\x7Fe"), "abcde");
        assert_eq!(sanitize("keep\t\n\r"), "keep\t\n\r");
        assert_eq!(sanitize("<script>&"), "<script>&");
    }
}
