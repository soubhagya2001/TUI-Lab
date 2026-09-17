//! `tuilab.yaml` parsing: defaults, strictness, security section.

use std::path::PathBuf;

use tui_lab_cli::config::load;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tuilab-cfg-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

#[test]
fn missing_config_gives_defaults() {
    let dir = scratch("missing");
    let config = load(&dir).expect("defaults");
    assert_eq!(config.tests_dir, "tests");
    assert_eq!(config.default_terminal.width, 120);
    assert_eq!(config.parallel, 4);
    assert_eq!(config.report.json, "reports/results.json");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn malformed_config_is_an_error() {
    let dir = scratch("malformed");
    std::fs::write(dir.join("tuilab.yaml"), "version: [unclosed\n").expect("write");
    assert!(load(&dir).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn full_config_parses_including_security() {
    let dir = scratch("full");
    std::fs::write(
        dir.join("tuilab.yaml"),
        r#"version: "1.0"
tests_dir: suites
default_terminal: { width: 80, height: 24 }
parallel: 2
security:
  allow_commands: ["./myapp", "cargo run*"]
"#,
    )
    .expect("write");
    let config = load(&dir).expect("parse");
    assert_eq!(config.tests_dir, "suites");
    assert_eq!(config.default_terminal.width, 80);
    assert_eq!(config.parallel, 2);
    assert_eq!(config.security.allow_commands.len(), 2);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn unknown_config_keys_are_rejected() {
    let dir = scratch("strict");
    std::fs::write(dir.join("tuilab.yaml"), "bogus_key: 1\n").expect("write");
    assert!(load(&dir).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}
