//! MCP security: command allowlist + cwd jail (docs/08 §8.4, DECIDED).
//!
//! * `tui_launch` only runs commands matching `security.allow_commands`
//!   (regex list from `tuilab.yaml`); anything else is rejected with
//!   [`crate::constants::FORBIDDEN_COMMAND`].
//! * `cwd` must resolve under the project root (`..` escape rejected,
//!   symlinks resolved).

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::constants::FORBIDDEN_COMMAND;

/// Compiled command allowlist.
#[derive(Clone, Debug)]
pub struct Allowlist {
    patterns: Vec<regex::Regex>,
    sources: Vec<String>,
}

impl Allowlist {
    /// Defaults from the spec: relative launches, `cargo run`, `python`.
    ///
    /// R11: patterns compile once behind an `OnceLock` (MSRV-clean, unlike
    /// `LazyLock`) — no per-call `expect` on the production path.
    pub fn defaults() -> Self {
        static DEFAULTS: OnceLock<Allowlist> = OnceLock::new();
        DEFAULTS
            .get_or_init(|| {
                // R11: no `expect` on the production path. These patterns are
                // crate constants, so a failure is a build defect — fail
                // closed (an empty allowlist denies every command) instead of
                // panicking the MCP server.
                match Allowlist::from_sources(&["^\\./.*", "^cargo run.*", "^python.*"]) {
                    Ok(allowlist) => allowlist,
                    Err(e) => {
                        tracing::error!("default allowlist failed to compile (deny-all): {e}");
                        Allowlist {
                            patterns: Vec::new(),
                            sources: Vec::new(),
                        }
                    }
                }
            })
            .clone()
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
///
/// R8: only a missing file falls back to defaults — other I/O failures are
/// errors, never silent.
pub fn load_allowlist(root: &Path) -> Result<Allowlist, String> {
    let path = root.join("tuilab.yaml");
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(format!("read {}: {e}", path.display())),
    };
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

/// Resolve a suite/file path (absolute, or relative to `root`) and confine
/// it under `root`. S3: `tui_run_test` must not read outside the project
/// (path traversal via `..` or absolute paths).
pub fn jail_file(root: &Path, file: &str) -> Result<PathBuf, String> {
    let canonical_root = root
        .canonicalize()
        .map_err(|e| format!("project root {}: {e}", root.display()))?;
    let candidate = {
        let path = Path::new(file);
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            canonical_root.join(path)
        }
    };
    let canonical = candidate
        .canonicalize()
        .map_err(|e| format!("suite file {}: {e}", candidate.display()))?;
    if !canonical.starts_with(&canonical_root) {
        return Err(format!(
            "suite file escapes project root: {}",
            canonical.display()
        ));
    }
    Ok(canonical)
}

/// One auditable field of an MCP call record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuditField {
    /// Milliseconds since the Unix epoch.
    Timestamp,
    /// Tool name (`tui_launch`, …).
    Tool,
    /// Session id, when the call targets one.
    Session,
    /// `ok` or the error string.
    Result,
    /// Tool arguments (see redaction rule below).
    Args,
    /// Milliseconds the call took.
    Elapsed,
}

impl AuditField {
    /// All selectable names, for error messages.
    pub const NAMES: &[&str] = &[
        "timestamp",
        "tool",
        "session_id",
        "result",
        "args",
        "elapsed_ms",
    ];

    fn parse(name: &str) -> Result<Self, String> {
        match name {
            "timestamp" => Ok(Self::Timestamp),
            "tool" => Ok(Self::Tool),
            "session_id" => Ok(Self::Session),
            "result" => Ok(Self::Result),
            "args" => Ok(Self::Args),
            "elapsed_ms" => Ok(Self::Elapsed),
            other => Err(format!(
                "unknown audit field {other:?}; select from {:?}",
                Self::NAMES
            )),
        }
    }
}

/// MCP call audit policy (`security.audit` in `tuilab.yaml`).
///
/// Disabled by default. When enabled, every tool call appends one JSON line.
/// `args` logging is sensitive-aware: `tui_type` text typed with
/// `sensitive: true` records as `"[redacted]"`, never the secret.
#[derive(Clone, Debug)]
pub struct AuditConfig {
    /// Master switch (default false).
    pub enabled: bool,
    /// JSONL sink (default `reports/mcp-audit.jsonl`, relative to root).
    pub path: PathBuf,
    /// Selected fields (default timestamp/tool/session_id/result).
    pub fields: Vec<AuditField>,
}

impl AuditConfig {
    /// Auditing off (the default).
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            path: PathBuf::from("reports/mcp-audit.jsonl"),
            fields: Self::default_fields(),
        }
    }

    /// Minimal fields logged when enabled without an explicit list.
    pub fn default_fields() -> Vec<AuditField> {
        vec![
            AuditField::Timestamp,
            AuditField::Tool,
            AuditField::Session,
            AuditField::Result,
        ]
    }
}

/// Load `security.audit` from `tuilab.yaml`; disabled when absent, error
/// when malformed (unknown field names included — typos must not silently
/// narrow the audit).
pub fn load_audit(root: &Path) -> Result<AuditConfig, String> {
    let path = root.join("tuilab.yaml");
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(format!("read {}: {e}", path.display())),
    };
    if text.trim().is_empty() {
        return Ok(AuditConfig::disabled());
    }
    let value: serde_yaml::Value =
        serde_yaml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let audit = match value
        .get("security")
        .and_then(|security| security.get("audit"))
    {
        None => return Ok(AuditConfig::disabled()),
        Some(audit) => audit,
    };
    let enabled = audit
        .get("enabled")
        .and_then(serde_yaml::Value::as_bool)
        .unwrap_or(false);
    let audit_path = audit
        .get("path")
        .and_then(serde_yaml::Value::as_str)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("reports/mcp-audit.jsonl"));
    let fields = match audit.get("fields") {
        None => AuditConfig::default_fields(),
        Some(list) => {
            let names = list
                .as_sequence()
                .ok_or_else(|| format!("{}: audit.fields must be a list", path.display()))?;
            let mut fields = Vec::with_capacity(names.len());
            for name in names {
                let name = name.as_str().ok_or_else(|| {
                    format!("{}: audit field names must be strings", path.display())
                })?;
                fields.push(AuditField::parse(name)?);
            }
            fields
        }
    };
    Ok(AuditConfig {
        enabled,
        path: audit_path,
        fields,
    })
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
    fn jail_file_confines_suites() {
        let root = std::env::temp_dir().join(format!("tuilab-jail-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("tests")).expect("scratch");
        let inside = root.join("tests").join("mini.yaml");
        std::fs::write(&inside, "schema: tui-lab/v1").expect("suite");
        // Relative inside-root resolves.
        let resolved = jail_file(&root, "tests/mini.yaml").expect("inside ok");
        assert_eq!(resolved, inside.canonicalize().expect("canonical"));
        // Absolute inside-root resolves.
        assert!(jail_file(&root, &inside.to_string_lossy()).is_ok());
        // `..` escape rejected even when it resolves to a real file.
        let outside =
            std::env::temp_dir().join(format!("tuilab-jail-out-{}.yaml", std::process::id()));
        std::fs::write(&outside, "schema: tui-lab/v1").expect("outside file");
        let dotdot = format!(
            "../{}",
            outside.file_name().expect("name").to_string_lossy()
        );
        let err = jail_file(&root, &dotdot).expect_err("dotdot blocked");
        assert!(err.contains("escapes project root"), "{err}");
        // Absolute outside rejected too.
        let err = jail_file(&root, &outside.to_string_lossy()).expect_err("outside blocked");
        assert!(err.contains("escapes project root"), "{err}");
        let _ = std::fs::remove_file(&outside);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_config_gives_defaults() {
        let dir = std::env::temp_dir().join("tuilab-mcp-no-config");
        let allow = load_allowlist(&dir).expect("defaults");
        assert!(allow.check("./myapp", &[]).is_ok());
    }

    #[test]
    fn unreadable_config_is_an_error() {
        // R8: only a *missing* file falls back to defaults.
        let dir =
            std::env::temp_dir().join(format!("tuilab-mcp-unreadable-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch");
        std::fs::create_dir_all(dir.join("tuilab.yaml")).expect("dir as file");
        let err = load_allowlist(&dir).expect_err("unreadable must fail");
        assert!(err.contains("read"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn audit_root(name: &str, yaml: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tuilab-audit-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch");
        std::fs::write(dir.join("tuilab.yaml"), yaml).expect("config");
        dir
    }

    #[test]
    fn audit_defaults_to_disabled() {
        let dir = std::env::temp_dir().join("tuilab-mcp-audit-absent");
        let cfg = load_audit(&dir).expect("absent ok");
        assert!(!cfg.enabled);
        assert_eq!(cfg.fields, AuditConfig::default_fields());
    }

    #[test]
    fn audit_parses_enabled_with_fields() {
        let dir = audit_root(
            "on",
            "security:\n  audit:\n    enabled: true\n    path: custom/audit.jsonl\n    fields: [timestamp, tool, args, elapsed_ms]\n",
        );
        let cfg = load_audit(&dir).expect("parse");
        assert!(cfg.enabled);
        assert_eq!(cfg.path, PathBuf::from("custom/audit.jsonl"));
        assert_eq!(
            cfg.fields,
            vec![
                AuditField::Timestamp,
                AuditField::Tool,
                AuditField::Args,
                AuditField::Elapsed,
            ]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn audit_rejects_unknown_fields() {
        // Typos must not silently narrow the audit.
        let dir = audit_root(
            "bad",
            "security:\n  audit:\n    enabled: true\n    fields: [timestamp, bogus]\n",
        );
        let err = load_audit(&dir).expect_err("unknown field fails");
        assert!(err.contains("bogus"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
