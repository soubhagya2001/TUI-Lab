//! `init` / `run` / `report` command handlers (docs/07).
//!
//! Thin over core + reporter: argument mapping, exit-code mapping, and the
//! human summary live here; step execution never does.

use std::path::{Path, PathBuf};

use tui_lab_core::{run_file, RunOptions, SuiteResult};
use tui_lab_protocol::TestFile;
use tui_lab_reporter::{load_json_all, to_junit_all, write_json_all};

use crate::config::{load, ProjectConfig};
use crate::constants::{
    CONFIG_FILE, EXIT_CONFIG_ERROR, EXIT_OK, EXIT_PTY_ERROR, EXIT_TESTS_FAILED, EXIT_TIMEOUT,
    TESTS_DIR,
};

/// Scaffold `tuilab.yaml` + `tests/smoke.yaml` in `dir`.
pub fn init(dir: &Path) -> i32 {
    let config_path = dir.join(CONFIG_FILE);
    if config_path.exists() {
        eprintln!(
            "{} already exists — leaving it alone",
            config_path.display()
        );
    } else {
        let config = ProjectConfig::default();
        let text = serde_yaml::to_string(&config).unwrap_or_default();
        if let Err(e) = std::fs::write(&config_path, text) {
            eprintln!("write {}: {e}", config_path.display());
            return EXIT_CONFIG_ERROR;
        }
        println!("wrote {}", config_path.display());
    }
    let tests_dir = dir.join(TESTS_DIR);
    if let Err(e) = std::fs::create_dir_all(&tests_dir) {
        eprintln!("create {}: {e}", tests_dir.display());
        return EXIT_CONFIG_ERROR;
    }
    let smoke = tests_dir.join("smoke.yaml");
    if !smoke.exists() {
        if let Err(e) = std::fs::write(&smoke, SMOKE_YAML) {
            eprintln!("write {}: {e}", smoke.display());
            return EXIT_CONFIG_ERROR;
        }
        println!("wrote {}", smoke.display());
    }
    EXIT_OK
}

/// Starter suite written by `init` (placeholder command for the user to edit).
const SMOKE_YAML: &str = r#"schema: tui-lab/v1
name: smoke
application:
  command: "./myapp"
terminal:
  width: 120
  height: 40
steps:
  - wait_for_text:
      text: "Main"
  - press: q
assertions:
  - exit_code: 0
"#;

/// Collect suite files: a single file, or all `*.yaml` under a directory
/// (recursive, sorted, so `tuilab run` finds every suite).
fn collect_suites(path: &Path) -> Result<Vec<PathBuf>, String> {
    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }
    if path.is_dir() {
        let mut files = Vec::new();
        collect_yaml_recursive(path, &mut files)?;
        files.sort();
        return Ok(files);
    }
    Err(format!("no such file or directory: {}", path.display()))
}

/// Depth-first `*.yaml` collection.
fn collect_yaml_recursive(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("read {}: {e}", dir.display()))?;
    for entry in entries {
        let item = entry
            .map_err(|e| format!("read {}: {e}", dir.display()))?
            .path();
        if item.is_file() && item.extension().is_some_and(|ext| ext == "yaml") {
            out.push(item);
        } else if item.is_dir() {
            collect_yaml_recursive(&item, out)?;
        }
    }
    Ok(())
}

/// Run suites sequentially; returns the CLI exit code.
///
/// `path` defaults to the configured `tests_dir` (`tuilab run` with no args).
pub async fn run(
    path: Option<&Path>,
    terminal_override: Option<(u16, u16)>,
    debug: bool,
    step_mode: bool,
) -> i32 {
    if step_mode {
        eprintln!("--step arrives with the interactive runner (v2); run without it for now");
        return EXIT_CONFIG_ERROR;
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let config = match load(&cwd) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("config error: {e}");
            return EXIT_CONFIG_ERROR;
        }
    };
    let suites = match path {
        Some(explicit) => collect_suites(explicit),
        None => collect_suites(Path::new(&config.tests_dir)),
    };
    let suites = match suites {
        Ok(suites) if !suites.is_empty() => suites,
        Ok(_) => {
            eprintln!("no suites found");
            return EXIT_CONFIG_ERROR;
        }
        Err(e) => {
            eprintln!("{e}");
            return EXIT_CONFIG_ERROR;
        }
    };

    let mut results = Vec::new();
    let mut exit = EXIT_OK;
    for suite_path in &suites {
        let text = match std::fs::read_to_string(suite_path) {
            Ok(text) => text,
            Err(e) => {
                eprintln!("read {}: {e}", suite_path.display());
                return EXIT_CONFIG_ERROR;
            }
        };
        let mut file = match TestFile::from_yaml(&text) {
            Ok(file) => file,
            Err(e) => {
                // Parse errors never launch anything (exit 2).
                eprintln!("{}: {e}", suite_path.display());
                return EXIT_CONFIG_ERROR;
            }
        };
        if let Some((width, height)) = terminal_override {
            file.terminal.width = width;
            file.terminal.height = height;
        }
        let opts = RunOptions {
            snapshot_dir: PathBuf::from(&config.snapshots_dir),
            ..RunOptions::default()
        };
        match run_file(&file, &opts).await {
            Ok(result) => {
                print_summary(&result);
                if debug {
                    print_debug(&result);
                }
                if !result.passed {
                    exit = EXIT_TESTS_FAILED;
                }
                results.push(result);
            }
            Err(e) => {
                // Infrastructure breakdown: launch/PTY/timeout (exits 3/4).
                eprintln!("{}: {e}", suite_path.display());
                let message = e.to_string();
                if message.starts_with("launch failed") || message.starts_with("pty failed") {
                    return EXIT_PTY_ERROR;
                }
                if message.starts_with("timed out") {
                    return EXIT_TIMEOUT;
                }
                return EXIT_TESTS_FAILED;
            }
        }
    }

    let report_path = PathBuf::from(&config.report.json);
    if let Err(e) = write_json_all(&report_path, &results) {
        eprintln!("write {}: {e}", report_path.display());
        return EXIT_CONFIG_ERROR;
    }
    let passed = results.iter().filter(|result| result.passed).count();
    println!("{passed} passed, {} failed", results.len() - passed);
    exit
}

/// One-line-per-suite human summary.
fn print_summary(result: &SuiteResult) {
    let mark = if result.passed { "✓" } else { "✗" };
    println!("{mark} {}", result.suite);
}

/// Debug dump: full failure bundle (screens, history, diffs).
fn print_debug(result: &SuiteResult) {
    if let Some(failure) = &result.failure {
        println!(
            "--- failure at step {} ({})",
            failure.step_index, failure.step
        );
        println!("expected: {}", failure.expected);
        println!("actual: {}", failure.actual);
        println!("input history:");
        for input in &failure.input_history {
            println!("  {input}");
        }
        println!("last screen:\n{}", failure.last_screen);
    }
    for step in &result.steps {
        println!(
            "  [{}] {} — {} ({}ms)",
            step.index, step.kind, step.detail, step.duration_ms
        );
    }
}

/// Re-render stored JSON results as JUnit.
pub fn report(format: &str, out: &Path, results_path: &Path) -> i32 {
    if format != "junit" {
        eprintln!("--format {format}: only junit for now (html arrives in v2)");
        return EXIT_CONFIG_ERROR;
    }
    let results = match load_json_all(results_path) {
        Ok(results) => results,
        Err(e) => {
            eprintln!("read {}: {e}", results_path.display());
            return EXIT_CONFIG_ERROR;
        }
    };
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("create {}: {e}", parent.display());
                return EXIT_CONFIG_ERROR;
            }
        }
    }
    match std::fs::write(out, to_junit_all(&results)) {
        Ok(()) => {
            println!("wrote {}", out.display());
            EXIT_OK
        }
        Err(e) => {
            eprintln!("write {}: {e}", out.display());
            EXIT_CONFIG_ERROR
        }
    }
}

/// `record` placeholder: the interactive recorder arrives in v2.
pub fn record() -> i32 {
    eprintln!("tuilab record arrives in v2 (see docs/10-recorder-and-ai.md)");
    eprintln!("({})", crate::utils::exit_message(EXIT_CONFIG_ERROR));
    EXIT_CONFIG_ERROR
}
