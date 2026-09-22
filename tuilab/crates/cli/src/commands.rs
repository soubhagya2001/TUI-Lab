//! `init` / `run` / `report` command handlers (docs/07).
//!
//! Thin over core + reporter: argument mapping, exit-code mapping, and the
//! human summary live here; step execution never does.

use std::path::{Path, PathBuf};

use tui_lab_core::{run_file_bounded, RunOptions, SuiteResult};
use tui_lab_protocol::TestFile;
use tui_lab_reporter::{load_json_all, write_json_all};

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
        // R8: a serialize failure must never write an empty yaml.
        let text = match serde_yaml::to_string(&config) {
            Ok(text) => text,
            Err(e) => {
                eprintln!("serialize default config: {e}");
                return EXIT_CONFIG_ERROR;
            }
        };
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

/// Collect suite files: a single file, or all `*.yaml`/`*.yml` under a
/// directory (recursive, sorted, so `tuilab run` finds every suite).
fn collect_suites(path: &Path) -> Result<Vec<PathBuf>, String> {
    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }
    if path.is_dir() {
        let mut files = Vec::new();
        let mut seen = Vec::new();
        collect_yaml_recursive(path, &mut files, &mut seen)?;
        files.sort();
        files.dedup();
        return Ok(files);
    }
    Err(format!("no such file or directory: {}", path.display()))
}

/// Depth-first suite collection.
///
/// R9: accepts `.yaml` and `.yml` (case-insensitive); canonicalizes every
/// directory to break symlink cycles and dedupes files reached twice.
fn collect_yaml_recursive(
    dir: &Path,
    out: &mut Vec<PathBuf>,
    seen: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let canonical = dir
        .canonicalize()
        .map_err(|e| format!("read {}: {e}", dir.display()))?;
    if seen.contains(&canonical) {
        return Ok(());
    }
    seen.push(canonical);
    let entries = std::fs::read_dir(dir).map_err(|e| format!("read {}: {e}", dir.display()))?;
    for entry in entries {
        let item = entry
            .map_err(|e| format!("read {}: {e}", dir.display()))?
            .path();
        if item.is_file() {
            let yaml = item.extension().is_some_and(|ext| {
                ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml")
            });
            if yaml {
                out.push(item);
            }
        } else if item.is_dir() {
            collect_yaml_recursive(&item, out, seen)?;
        }
    }
    Ok(())
}

/// Run suites (sequentially or in parallel); returns the CLI exit code.
///
/// `path` defaults to the configured `tests_dir` (`tuilab run` with no args).
/// `parallel` defaults to the configured slot count; 1 keeps the exact
/// sequential behavior (including immediate infra-error exits).
pub async fn run(
    path: Option<&Path>,
    terminal_override: Option<(u16, u16)>,
    debug: bool,
    step_mode: bool,
    parallel: Option<usize>,
) -> i32 {
    if step_mode {
        eprintln!("step mode: Enter continues each step, q aborts the run");
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
    let requested = parallel.unwrap_or(config.parallel);
    let mut slots = requested.clamp(1, tui_lab_core::sessions::DEFAULT_MAX_SESSIONS);
    // R9: clamping is visible, never silent.
    if slots != requested {
        eprintln!(
            "warning: parallel slots clamped to {slots} (registry cap {})",
            tui_lab_core::sessions::DEFAULT_MAX_SESSIONS
        );
    }
    if step_mode && slots != 1 {
        // Pausing across parallel tasks on shared stdin is incoherent.
        eprintln!("--step implies sequential runs (slots forced to 1)");
        slots = 1;
    }
    if slots == 1 {
        for suite_path in &suites {
            let (file, opts) = match load_suite(suite_path, &config, terminal_override, step_mode) {
                Ok(loaded) => loaded,
                Err(code) => return code,
            };
            match run_file_bounded(&file, &opts).await {
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
                    return exit_for(&e);
                }
            }
        }
    } else {
        // Parallel: parse everything first so a schema error still exits 2
        // before anything launches; then fan out with run-all semantics
        // (infra errors arrive as failed results, never aborts). Each suite
        // keeps its wired options (C5: timeouts survive fan-out).
        let mut files = Vec::with_capacity(suites.len());
        for suite_path in &suites {
            match load_suite(suite_path, &config, terminal_override, step_mode) {
                Ok(loaded) => files.push(loaded),
                Err(code) => return code,
            }
        }
        for result in tui_lab_core::run_suites(files, slots).await {
            print_summary(&result);
            if debug {
                print_debug(&result);
            }
            if !result.passed {
                exit = EXIT_TESTS_FAILED;
            }
            results.push(result);
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

/// Map infrastructure errors to CLI exit codes.
///
/// R5: matches the `CoreError` enum, never message prefixes — rewording an
/// error message must not silently change exits.
fn exit_for(error: &tui_lab_core::error::CoreError) -> i32 {
    use tui_lab_core::error::CoreError as E;
    match error {
        E::Launch(_) | E::Pty(_) => EXIT_PTY_ERROR,
        E::Timeout(_) => EXIT_TIMEOUT,
        E::Message(_) => EXIT_TESTS_FAILED,
    }
}

/// Read, parse, and tune one suite file. Errors exit before any launch.
fn load_suite(
    suite_path: &Path,
    config: &crate::config::ProjectConfig,
    terminal_override: Option<(u16, u16)>,
    step_mode: bool,
) -> Result<(TestFile, RunOptions), i32> {
    let text = std::fs::read_to_string(suite_path).map_err(|e| {
        eprintln!("read {}: {e}", suite_path.display());
        EXIT_CONFIG_ERROR
    })?;
    let mut file = TestFile::from_yaml(&text).map_err(|e| {
        // Parse errors never launch anything (exit 2).
        eprintln!("{}: {e}", suite_path.display());
        EXIT_CONFIG_ERROR
    })?;
    if let Some((width, height)) = terminal_override {
        file.terminal.width = width;
        file.terminal.height = height;
    }
    // C5: wire the parsed-but-ignored settings. Config terminal geometry
    // applies only when the suite (and CLI flags) left the default 120x40;
    // config env fills gaps the suite did not set; config default_timeout
    // becomes the wait default (malformed values are config errors).
    if file.terminal == tui_lab_protocol::TerminalConfig::default() {
        file.terminal.width = config.default_terminal.width;
        file.terminal.height = config.default_terminal.height;
    }
    for (key, value) in &config.env {
        file.environment
            .entry(key.clone())
            .or_insert_with(|| value.clone());
    }
    let wait_default = tui_lab_protocol::parse_duration(&config.default_timeout).map_err(|e| {
        eprintln!("bad default_timeout {:?}: {e}", config.default_timeout);
        EXIT_CONFIG_ERROR
    })?;
    let opts = RunOptions {
        snapshot_dir: PathBuf::from(&config.snapshots_dir),
        wait_default,
        step_hook: step_mode.then(step_pause_hook),
        ..RunOptions::default()
    };
    Ok((file, opts))
}

/// Pause hook for `--step`: Enter continues, q aborts. Closed stdin
/// (pipes, CI) reads EOF at once, so headless runs never hang here.
fn step_pause_hook() -> tui_lab_core::StepHook {
    std::sync::Arc::new(|result: &tui_lab_core::StepResult| {
        println!(
            "[step {}] {} — {}",
            result.index, result.kind, result.detail
        );
        println!("Enter to continue, q to abort: ");
        let mut line = String::new();
        match std::io::stdin().read_line(&mut line) {
            Ok(_) => !line.trim().eq_ignore_ascii_case("q"),
            Err(_) => true,
        }
    })
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

/// Re-render stored JSON results as JUnit or self-contained HTML.
pub fn report(format: &str, out: &Path, results_path: &Path) -> i32 {
    let render: fn(&[tui_lab_core::SuiteResult]) -> String = match format {
        "junit" => tui_lab_reporter::to_junit_all,
        "html" => tui_lab_reporter::to_html,
        _ => {
            eprintln!("--format {format}: expected junit or html");
            return EXIT_CONFIG_ERROR;
        }
    };
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
    match std::fs::write(out, render(&results)) {
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

/// Record an interactive session to a YAML suite (docs/10).
///
/// Thin adapter: argument mapping lives here, capture + synthesis in
/// [`crate::recorder`].
pub fn record(
    command: Option<String>,
    args: Vec<String>,
    out: Option<PathBuf>,
    terminal: Option<(u16, u16)>,
) -> i32 {
    let Some(command) = command else {
        eprintln!("record needs --command <binary> (see `tuilab record --help`)");
        return EXIT_CONFIG_ERROR;
    };
    let out = out.unwrap_or_else(|| {
        let stem = Path::new(&command)
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| "recording".to_string());
        PathBuf::from(format!("{stem}-record.yaml"))
    });
    let (width, height) = terminal.unwrap_or((120, 40));
    crate::recorder::run(&command, &args, &out, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R5: exit codes follow the error enum, not message text.
    #[test]
    fn exit_codes_match_error_variants() {
        use tui_lab_core::error::CoreError as E;
        assert_eq!(exit_for(&E::Launch("x".into())), EXIT_PTY_ERROR);
        assert_eq!(exit_for(&E::Pty("x".into())), EXIT_PTY_ERROR);
        assert_eq!(exit_for(&E::Timeout("x".into())), EXIT_TIMEOUT);
        // Even a timeout-sounding free message stays exit 1: only the
        // Timeout variant means exit 4.
        assert_eq!(
            exit_for(&E::Message("timed out waiting".into())),
            EXIT_TESTS_FAILED
        );
    }

    /// R9: `.yaml` + `.yml` discovered, others ignored.
    #[test]
    fn discovery_accepts_both_yaml_extensions() {
        let dir = std::env::temp_dir().join(format!("tuilab-discover-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("nested")).expect("scratch");
        for name in ["a.yaml", "b.yml", "c.txt", "nested/d.yaml"] {
            std::fs::write(dir.join(name), "x").expect("suite file");
        }
        let mut found: Vec<String> = collect_suites(&dir)
            .expect("collect")
            .iter()
            .map(|path| {
                path.strip_prefix(&dir)
                    .expect("under root")
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        found.sort();
        assert_eq!(found, ["a.yaml", "b.yml", "nested/d.yaml"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// R9: symlink cycles terminate instead of recursing forever.
    #[test]
    fn discovery_survives_symlink_cycles() {
        let dir = std::env::temp_dir().join(format!("tuilab-cycle-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let sub = dir.join("sub");
        std::fs::create_dir_all(&sub).expect("scratch");
        std::fs::write(dir.join("a.yaml"), "x").expect("suite file");
        #[cfg(windows)]
        let linked = std::os::windows::fs::symlink_dir(&dir, sub.join("loop"));
        #[cfg(not(windows))]
        let linked = std::os::unix::fs::symlink(&dir, sub.join("loop"));
        if linked.is_err() {
            eprintln!("warning: symlinks need privilege; skipping cycle test");
            let _ = std::fs::remove_dir_all(&dir);
            return;
        }
        let found = collect_suites(&dir).expect("collect terminates");
        assert_eq!(found.len(), 1, "no duplicates through the loop");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
