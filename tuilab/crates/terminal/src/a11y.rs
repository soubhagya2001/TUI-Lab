//! Heuristic accessibility tree from the rendered grid (P5-E1).
//!
//! No app cooperation: roles are inferred from widespread TUI conventions
//! (`[ Submit ]` buttons, `Search: ___` inputs, `[x]` checkboxes). This is
//! intentionally approximate — a targeting aid for agents and `role`
//! assertions, not a screen reader. Custom-drawn widgets that ignore these
//! conventions are invisible to the tree (documented limitation).

/// Widget role inferred from rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// `[ Label ]` or `< Label >`.
    Button,
    /// `Label: ___` (or trailing underscores).
    TextInput,
    /// `[x]` / `[ ]` (any case).
    Checkbox,
}

impl Role {
    /// Canonical lowercase name (wire + YAML + JSON).
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Button => "button",
            Self::TextInput => "textinput",
            Self::Checkbox => "checkbox",
        }
    }

    /// Parse a canonical role name.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "button" => Some(Self::Button),
            "textinput" => Some(Self::TextInput),
            "checkbox" => Some(Self::Checkbox),
            _ => None,
        }
    }
}

/// One inferred widget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct A11yNode {
    /// Widget role.
    pub role: Role,
    /// Visible label.
    pub name: String,
    /// Zero-based column of the widget start.
    pub x: usize,
    /// Zero-based row.
    pub y: usize,
    /// Whether the cursor sits inside the widget span.
    pub focused: bool,
}

/// Find the first node with `role` whose name contains `needle`
/// (case-insensitive).
#[must_use]
pub fn find_role<'a>(nodes: &'a [A11yNode], role: Role, needle: &str) -> Option<&'a A11yNode> {
    let needle = needle.to_lowercase();
    nodes
        .iter()
        .find(|node| node.role == role && node.name.to_lowercase().contains(&needle))
}

/// Build a tree from plain-text rows plus the cursor position.
///
/// `rows` are visible grid lines (trailing padding already trimmed, as from
/// [`crate::Emulator::text`]); `cursor` is zero-based `(row, col)`.
#[must_use]
pub fn build_tree(rows: &[String], cursor: (usize, usize)) -> Vec<A11yNode> {
    let mut nodes = Vec::new();
    for (y, row) in rows.iter().enumerate() {
        scan_row(row, y, cursor, &mut nodes);
    }
    nodes
}

/// Render a tree in the stable dump format (goldens, agent context).
#[must_use]
pub fn dump_tree(nodes: &[A11yNode]) -> String {
    nodes
        .iter()
        .map(|node| {
            let mut line = format!(
                "{} {:?} at ({},{})",
                node.role.name(),
                node.name,
                node.x,
                node.y
            );
            if node.focused {
                line.push_str(" [focused]");
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn in_span(start: usize, end: usize, y: usize, cursor: (usize, usize)) -> bool {
    cursor.0 == y && cursor.1 >= start && cursor.1 < end
}

fn scan_row(row: &str, y: usize, cursor: (usize, usize), nodes: &mut Vec<A11yNode>) {
    let chars: Vec<char> = row.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        // Checkbox: [x] / [ ] / [X].
        if chars[i] == '['
            && i + 2 < chars.len()
            && (chars[i + 1] == 'x' || chars[i + 1] == 'X' || chars[i + 1] == ' ')
            && chars[i + 2] == ']'
        {
            let name = chars[i + 3..].iter().collect::<String>().trim().to_string();
            nodes.push(A11yNode {
                role: Role::Checkbox,
                name,
                x: i,
                y,
                focused: in_span(i, i + 3, y, cursor),
            });
            i += 3;
            continue;
        }
        // Button: [ Label ] (non-checkbox brackets).
        if chars[i] == '[' {
            if let Some(rel) = chars[i..].iter().position(|&c| c == ']') {
                let inner: String = chars[i + 1..i + rel].iter().collect();
                let name = inner.trim().to_string();
                if !name.is_empty() && name.len() < 40 {
                    nodes.push(A11yNode {
                        role: Role::Button,
                        name,
                        x: i,
                        y,
                        focused: in_span(i, i + rel + 1, y, cursor),
                    });
                    i += rel + 1;
                    continue;
                }
            }
        }
        // Button: < Label >.
        if chars[i] == '<' {
            if let Some(rel) = chars[i..].iter().position(|&c| c == '>') {
                let inner: String = chars[i + 1..i + rel].iter().collect();
                let name = inner.trim().to_string();
                if !name.is_empty() && name.len() < 40 && !inner.contains('<') {
                    nodes.push(A11yNode {
                        role: Role::Button,
                        name,
                        x: i,
                        y,
                        focused: in_span(i, i + rel + 1, y, cursor),
                    });
                    i += rel + 1;
                    continue;
                }
            }
        }
        i += 1;
    }
    // Text input: `Label: ___` or a trailing run of 2+ underscores.
    let trimmed = row.trim_end();
    if let Some(colon) = trimmed.find(':') {
        let after = trimmed[colon + 1..].trim();
        if !after.is_empty() && after.chars().all(|c| c == '_' || c == ' ') {
            let name = trimmed[..=colon].trim().to_string();
            if !name.is_empty() {
                let x = row.find(&name).unwrap_or(0);
                nodes.push(A11yNode {
                    role: Role::TextInput,
                    name,
                    x,
                    y,
                    focused: cursor.0 == y,
                });
                return;
            }
        }
    }
    if trimmed.len() >= 2 && trimmed.ends_with("__") {
        let stem = trimmed.trim_end_matches('_').trim_end().to_string();
        if !stem.is_empty() {
            let x = row.find(&stem).unwrap_or(0);
            nodes.push(A11yNode {
                role: Role::TextInput,
                name: stem,
                x,
                y,
                focused: cursor.0 == y,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(text: &str, cursor: (usize, usize)) -> Vec<A11yNode> {
        build_tree(
            &text.lines().map(str::to_string).collect::<Vec<_>>(),
            cursor,
        )
    }

    #[test]
    fn finds_buttons_inputs_and_checkboxes() {
        let nodes = tree(
            "[ Submit ]\nSearch: ___\n[x] Remember me\nplain text",
            (1, 9),
        );
        assert_eq!(nodes.len(), 3);
        assert_eq!(nodes[0].role, Role::Button);
        assert_eq!(nodes[0].name, "Submit");
        assert!(!nodes[0].focused);
        assert_eq!(nodes[1].role, Role::TextInput);
        assert_eq!(nodes[1].name, "Search:");
        assert!(nodes[1].focused, "cursor on the input row");
        assert_eq!(nodes[2].role, Role::Checkbox);
        assert_eq!(nodes[2].name, "Remember me");
    }

    #[test]
    fn angle_buttons_and_unchecked_boxes() {
        let nodes = tree("< Cancel >\n[ ] Subscribe", (0, 0));
        assert_eq!(nodes.len(), 2);
        assert!(nodes
            .iter()
            .any(|n| n.role == Role::Button && n.name == "Cancel"));
        assert!(nodes.iter().any(|n| n.role == Role::Checkbox));
    }

    #[test]
    fn plain_text_yields_no_nodes() {
        assert!(tree("Hello world\nStatus: OK", (0, 0)).is_empty());
    }

    #[test]
    fn find_role_matches_case_insensitively() {
        let nodes = tree("[ SUBMIT ]", (0, 0));
        assert!(find_role(&nodes, Role::Button, "submit").is_some());
        assert!(find_role(&nodes, Role::Button, "cancel").is_none());
        assert!(find_role(&nodes, Role::TextInput, "submit").is_none());
    }

    #[test]
    fn dump_is_stable() {
        let nodes = tree("[ OK ]", (0, 1));
        assert_eq!(dump_tree(&nodes), "button \"OK\" at (0,0) [focused]");
    }

    #[test]
    fn role_names_round_trip() {
        for role in [Role::Button, Role::TextInput, Role::Checkbox] {
            assert_eq!(Role::parse(role.name()), Some(role));
        }
        assert_eq!(Role::parse("slider"), None);
    }
}
