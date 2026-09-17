//! Pure snapshot-path helpers.

/// Golden-file path for a named snapshot at a given size (docs/06 §6.2).
#[must_use]
pub fn snapshot_file(dir: &str, name: &str, width: u16, height: u16) -> String {
    format!("{dir}/{name}/{width}x{height}.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_size_scoped_path() {
        assert_eq!(
            snapshot_file("tests/snapshots", "dashboard", 80, 24),
            "tests/snapshots/dashboard/80x24.json"
        );
    }
}
