//! Pure screen-model helpers.

use crate::constants::{MAX_HEIGHT, MAX_WIDTH};

/// Clamp requested dimensions to the materialization limits.
#[must_use]
pub fn clamp_dims(width: u16, height: u16) -> (u16, u16) {
    (width.min(MAX_WIDTH), height.min(MAX_HEIGHT))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_dims_pass_through() {
        assert_eq!(clamp_dims(120, 40), (120, 40));
    }

    #[test]
    fn hostile_dims_are_clamped() {
        assert_eq!(clamp_dims(u16::MAX, u16::MAX), (MAX_WIDTH, MAX_HEIGHT));
    }
}

/// Join grid rows into screen text, trimming trailing whitespace per row.
///
/// Pure so assertions and reporters can share it without a live emulator.
#[must_use]
pub fn render_text(rows: &[Vec<char>]) -> String {
    let mut out = String::new();
    for row in rows {
        let line: String = row.iter().collect();
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod render_tests {
    use super::*;

    #[test]
    fn trims_trailing_padding() {
        let rows = vec![vec!['H', 'i', ' ', ' '], vec![' ', ' ', ' ']];
        assert_eq!(render_text(&rows), "Hi\n\n");
    }
}
