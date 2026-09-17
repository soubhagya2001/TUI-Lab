//! MCP security: command allowlist + cwd jail (docs/08 §8.4, DECIDED).
//!
//! * `tui_launch` only runs commands matching `security.allow_commands`
//!   (regex list from `tuilab.yaml`); anything else is rejected with
//!   [`crate::constants::FORBIDDEN_COMMAND`].
//! * `cwd` must resolve under the project root (`..` escape rejected,
//!   symlinks resolved).

use std::path::{Path, PathBuf};

use crate::constants::FORBIDDEN_COMMAND;

/// Compiled command allowlist.
pub struct Allowlist {
    patterns: Vec<regex::Regex>,
    sources: Vec<String>,
}

impl Allowlist {
    /// Defaults from the spec: relative launches, `cargo run`, `python`.
    pub fn defaults() -> Self {
        Self::from_sources(&["^\\./.*", "^cargo run.*", "^python.*"])
            .expect("default allowlist compiles")
    }

    /// Compile user patterns; invalid regex is an error, never silent.
    pub fn from_sources(sources: &[&str]) -> Result<Self, String> {
        let mut patterns = Vec::with_capacity(sources.len());
        for source in sources {
            let pattern = regex::Regex::new(source)
                .map_err(|e| format!("bad allow_commands pattern {source:?}: {e}"))?;
            patterns.push(pattern);
        }
        Ok(Self {
            patterns,
            sources: sources.iter().map(|source| (*source).to_owned()).collect(),
        })
    }

    /// Check `command + args` against the list.
    pub fn check(&self, command: &str, args: &[String]) -> Result<(), String> {
        let line = if args.is_empty() {
            command.to_string()
        } else {
            format!("{command} {}", args.join(" "))
        };
        if self.patterns.iter().any(|pattern| pattern.is_match(&line)) {
            Ok(())
        } else {
            Err(format!(
                "{FORBIDDEN_COMMAND}: {line:?} matches none of {:?}",
                self.sources
            ))
        }
    }
}

/// Load `security.allow_commands` from `tuilab.yaml`; defaults when absent,
/// error when malformed.
pub fn load_allowlist(root: &Path) -> Result<Allowlist, String> {
    let path = root.join("tuilab.yaml");
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    if text.trim().is_empty() {
        return Ok(Allowlist::defaults());
    }
    let value: serde_yaml::Value =
        serde_yaml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let entries = value
        .get("security")
        .and_then(|security| security.get("allow_commands"))
        .and_then(serde_yaml::Value::as_sequence)
        .map(|seq| {
            seq.iter()
                .filter_map(serde_yaml::Value::as_str)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if entries.is_empty() {
        return Ok(Allowlist::defaults());
    }
    Allowlist::from_sources(&entries)
}

/// Resolve `cwd` (relative to `root`) and confine it under `root`.
pub fn jail(root: &Path, cwd: Option<&str>) -> Result<PathBuf, String> {
    let canonical_root = root
        .canonicalize()
        .map_err(|e| format!("project root {}: {e}", root.display()))?;
    let joined = match cwd {
        Some(dir) => canonical_root.join(dir),
        None => canonical_root.clone(),
    };
    let canonical = joined
        .canonicalize()
        .map_err(|e| format!("cwd {}: {e}", joined.display()))?;
    if !canonical.starts_with(&canonical_root) {
        return Err(format!("cwd escapes project root: {}", canonical.display()));
    }
    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_allow_documented_commands() {
        let allow = Allowlist::defaults();
        assert!(allow.check("./myapp", &[]).is_ok());
        assert!(allow.check("cargo", &["run".into()]).is_ok());
        assert!(allow.check("python", &["app.py".into()]).is_ok());
    }

    #[test]
    fn unmatched_commands_are_forbidden() {
        let allow = Allowlist::defaults();
        let err = allow
            .check("rm", &["-rf".into(), "/".into()])
            .expect_err("rm is forbidden");
        assert!(err.starts_with(FORBIDDEN_COMMAND));
    }

    #[test]
    fn bad_patterns_are_errors() {
        assert!(Allowlist::from_sources(&["(["]).is_err());
    }

    #[test]
    fn custom_patterns_replace_defaults() {
        let allow = Allowlist::from_sources(&["^\\.\\/myapp$"]).expect("compile");
        assert!(allow.check("./myapp", &[]).is_ok());
        assert!(allow.check("./other", &[]).is_err());
    }

    #[test]
    fn jail_blocks_dotdot_escape() {
        let root = std::env::temp_dir();
        assert!(jail(&root, Some("../..")).is_err());
        assert!(jail(&root, None).is_ok());
    }

    #[test]
    fn missing_config_gives_defaults() {
        let dir = std::env::temp_dir().join("tuilab-mcp-no-config");
        let allow = load_allowlist(&dir).expect("defaults");
        assert!(allow.check("./myapp", &[]).is_ok());
    }
}
