//! `tuilab.yaml` project config (docs/07 §7.2).
//!
//! Parsed strictly (unknown fields denied); the `security` section is data
//! for now — enforcement arrives with the MCP server in Phase 4.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::constants::CONFIG_FILE;

/// Project configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    /// Config version.
    #[serde(default = "default_version")]
    pub version: String,
    /// Directory holding `*.yaml` suites.
    #[serde(default = "default_tests_dir")]
    pub tests_dir: String,
    /// Golden snapshots directory.
    #[serde(default = "default_snapshots_dir")]
    pub snapshots_dir: String,
    /// Default terminal geometry.
    #[serde(default)]
    pub default_terminal: TerminalSize,
    /// Default per-wait timeout (`10s`).
    #[serde(default = "default_timeout")]
    pub default_timeout: String,
    /// Parallel slots (v2 honors this; Phase 3 runs sequentially).
    #[serde(default = "default_parallel")]
    pub parallel: usize,
    /// Report output paths.
    #[serde(default)]
    pub report: ReportPaths,
    /// Extra environment for children.
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// MCP security policy (parsed, enforced in Phase 4).
    #[serde(default)]
    pub security: SecurityPolicy,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            version: default_version(),
            tests_dir: default_tests_dir(),
            snapshots_dir: default_snapshots_dir(),
            default_terminal: TerminalSize::default(),
            default_timeout: default_timeout(),
            parallel: default_parallel(),
            report: ReportPaths::default(),
            env: HashMap::new(),
            security: SecurityPolicy::default(),
        }
    }
}

/// Terminal geometry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalSize {
    /// Columns.
    pub width: u16,
    /// Rows.
    pub height: u16,
}

impl Default for TerminalSize {
    fn default() -> Self {
        Self {
            width: 120,
            height: 40,
        }
    }
}

/// Report output paths.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportPaths {
    /// JSON results path.
    pub json: String,
    /// JUnit XML path.
    pub junit: String,
    /// HTML report path (v2).
    pub html: String,
}

impl Default for ReportPaths {
    fn default() -> Self {
        Self {
            json: "reports/results.json".to_string(),
            junit: "reports/junit.xml".to_string(),
            html: "reports/index.html".to_string(),
        }
    }
}

/// MCP security policy (parsed now, enforced in Phase 4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityPolicy {
    /// Regex allowlist for `tui_launch` commands.
    #[serde(default)]
    pub allow_commands: Vec<String>,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            allow_commands: vec!["./*".to_string()],
        }
    }
}

fn default_version() -> String {
    "1.0".to_string()
}

fn default_tests_dir() -> String {
    "tests".to_string()
}

fn default_snapshots_dir() -> String {
    "tests/snapshots".to_string()
}

fn default_timeout() -> String {
    "10s".to_string()
}

fn default_parallel() -> usize {
    4
}

/// Load `tuilab.yaml` from `dir` (defaults when absent, error when malformed).
pub fn load(dir: &Path) -> Result<ProjectConfig, String> {
    let path = dir.join(CONFIG_FILE);
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    if text.trim().is_empty() {
        return Ok(ProjectConfig::default());
    }
    serde_yaml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}
