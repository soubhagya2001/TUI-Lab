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

/// Minimal unified diff: common prefix/suffix trimmed, the changed middle
/// shown as `-` (expected) / `+` (actual) lines with `@@` hunk header.
#[must_use]
pub fn unified_diff(expected: &str, actual: &str) -> String {
    let expected_lines: Vec<&str> = expected.lines().collect();
    let actual_lines: Vec<&str> = actual.lines().collect();
    let mut prefix = 0;
    while prefix < expected_lines.len()
        && prefix < actual_lines.len()
        && expected_lines[prefix] == actual_lines[prefix]
    {
        prefix += 1;
    }
    let mut suffix = 0;
    while suffix < expected_lines.len() - prefix
        && suffix < actual_lines.len() - prefix
        && expected_lines[expected_lines.len() - 1 - suffix]
            == actual_lines[actual_lines.len() - 1 - suffix]
    {
        suffix += 1;
    }
    let mut out = format!(
        "@@ -{},{} +{},{} @@\n",
        prefix + 1,
        expected_lines.len() - prefix - suffix,
        prefix + 1,
        actual_lines.len() - prefix - suffix
    );
    for line in &expected_lines[prefix..expected_lines.len() - suffix] {
        out.push_str("- ");
        out.push_str(line);
        out.push('\n');
    }
    for line in &actual_lines[prefix..actual_lines.len() - suffix] {
        out.push_str("+ ");
        out.push_str(line);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod diff_tests {
    use super::*;

    #[test]
    fn diff_highlights_changed_lines() {
        let diff = unified_diff("a\nb\nc", "a\nB\nc");
        assert!(diff.contains("- b"));
        assert!(diff.contains("+ B"));
        assert!(!diff.contains("- a"));
    }

    #[test]
    fn identical_text_has_empty_hunks() {
        let diff = unified_diff("a\nb", "a\nb");
        assert!(!diff.contains("- "));
        assert!(!diff.contains("+ "));
    }
}
