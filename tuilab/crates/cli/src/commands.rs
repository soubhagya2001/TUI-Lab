//! `init` / `run` / `report` command handlers (docs/07).
//!
//! Thin over core + reporter: argument mapping, exit-code mapping, and the
//! human summary live here; step execution never does.

use std::path::{Path, PathBuf};

use crate::config::{load, ProjectConfig};
use crate::constants::{
    CONFIG_FILE, EXIT_CONFIG_ERROR, EXIT_OK, EXIT_PTY_ERROR, EXIT_TESTS_FAILED, EXIT_TIMEOUT,
    TESTS_DIR,
};
use tui_lab_core::{run_file_bounded, RunOptions, SuiteResult, TerminalInfo};
use tui_lab_protocol::TestFile;
use tui_lab_reporter::{
    load_json_all, read_trace, render_timeline, replay_schedule, write_json_all, write_trace,
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

/// Arguments for [`run`] (bundled so the signature stays lint-clean).
pub struct RunArgs<'a> {
    /// Suite file or directory (defaults to configured `tests_dir`).
    pub path: Option<&'a Path>,
    /// Terminal override, e.g. 120x40.
    pub terminal_override: Option<(u16, u16)>,
    /// Print the full failure bundle on failure.
    pub debug: bool,
    /// Step-through mode.
    pub step_mode: bool,
    /// Parallel slots (default: `tuilab.yaml` `parallel`).
    pub parallel: Option<usize>,
    /// Shard selection N/M (both 1-based).
    pub shard: Option<(usize, usize)>,
    /// Rerun failed suites up to N extra times.
    pub retries: usize,
    /// Run only suites carrying any of these tags.
    pub tags: &'a [String],
    /// Trace capture: `Some("always")` writes a trace.zip per suite,
    /// `Some("never")` disables; `None` keeps retain-on-failure.
    pub trace: Option<&'a str>,
}

/// Run suites (sequentially or in parallel); returns the CLI exit code.
///
/// `parallel` 1 keeps the exact sequential behavior (including immediate
/// infra-error exits).
/// `shard` selects the Nth slice of M, `retries` reruns failures,
/// non-empty `tags` keeps only matching suites.
pub async fn run(args: RunArgs<'_>) -> i32 {
    let RunArgs {
        path,
        terminal_override,
        debug,
        step_mode,
        parallel,
        shard,
        retries,
        tags,
        trace,
    } = args;
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

    // Parse everything first so a schema error still exits 2 before anything
    // launches; then select (skip/focus/tags/shard).
    let mut pairs = Vec::with_capacity(suites.len());
    for suite_path in &suites {
        match load_suite(suite_path, &config, terminal_override, step_mode) {
            Ok(pair) => pairs.push(pair),
            Err(code) => return code,
        }
    }
    // Declared attachments per suite (names unique in practice; first wins).
    let mut declared: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (file, _) in &pairs {
        declared
            .entry(file.name.clone())
            .or_insert_with(|| file.attachments.clone());
    }
    let (run_list, mut ordered) = select_suites(pairs, tags);
    // `ordered` carries selection indices so reports reassemble in input
    // order even when retries land late.
    if run_list.is_empty() {
        eprintln!("no suites selected (tags/shard/focus filters)");
    }
    let mut run_list = run_list;
    if let Some((n, m)) = shard {
        run_list = apply_shard(run_list, n, m);
        if run_list.is_empty() {
            eprintln!("no suites in shard {n}/{m}");
        }
    }

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
        for (index, file, opts) in run_list {
            // Retries rerun failures fresh; only the final attempt lands in
            // the report (its `attempts` count tells the story). Infra
            // errors still exit at once — a broken environment won't heal
            // by retrying.
            let mut attempt = 0u32;
            loop {
                attempt += 1;
                match run_file_bounded(&file, &opts).await {
                    Ok(mut result) => {
                        result.attempts = attempt;
                        let failed = !result.passed && !result.skipped;
                        if !failed || attempt > retries as u32 {
                            print_summary(&result);
                            if debug {
                                print_debug(&result);
                            }
                            if failed {
                                exit = EXIT_TESTS_FAILED;
                            }
                            ordered.push((index, result));
                            break;
                        }
                        eprintln!("retrying {} (attempt {})", file.name, attempt + 1);
                    }
                    Err(e) => {
                        // Infrastructure breakdown: launch/PTY/timeout (exits 3/4).
                        eprintln!("{}: {e}", file.name);
                        return exit_for(&e);
                    }
                }
            }
        }
    } else {
        // Parallel fan-out with run-all semantics (infra errors arrive as
        // failed results, never aborts). Each suite keeps its wired options
        // (C5: timeouts survive fan-out). Failed suites requeue by pool
        // position until attempts run out; finals merge back by selection
        // index so reports stay in input order.
        let pool: Vec<IndexedSuite> = run_list;
        let mut pending: Vec<usize> = (0..pool.len()).collect();
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            let round: Vec<(TestFile, RunOptions)> = pending
                .iter()
                .map(|&i| (pool[i].1.clone(), pool[i].2.clone()))
                .collect();
            let batch = tui_lab_core::run_suites(round, slots).await;
            let mut requeue = Vec::new();
            for (pos, mut result) in batch.into_iter().enumerate() {
                result.attempts = attempt;
                let failed = !result.passed && !result.skipped;
                if failed && attempt <= retries as u32 {
                    eprintln!("retrying {} (attempt {})", result.suite, attempt + 1);
                    requeue.push(pending[pos]);
                    continue;
                }
                print_summary(&result);
                if debug {
                    print_debug(&result);
                }
                if failed {
                    exit = EXIT_TESTS_FAILED;
                }
                ordered.push((pool[pending[pos]].0, result));
            }
            if requeue.is_empty() {
                break;
            }
            pending = requeue;
        }
    }

    ordered.sort_by_key(|(index, _)| *index);
    let mut results: Vec<SuiteResult> = ordered.into_iter().map(|(_, result)| result).collect();

    collect_attachments(&mut results, &declared);
    append_history(&results);
    write_traces(&results, trace);

    let report_path = PathBuf::from(&config.report.json);
    if let Err(e) = write_json_all(&report_path, &results) {
        eprintln!("write {}: {e}", report_path.display());
        return EXIT_CONFIG_ERROR;
    }
    let passed = results
        .iter()
        .filter(|result| result.passed && !result.skipped)
        .count();
    let failed = results
        .iter()
        .filter(|result| !result.passed && !result.skipped)
        .count();
    let skipped = results.iter().filter(|result| result.skipped).count();
    if skipped == 0 {
        println!("{passed} passed, {failed} failed");
    } else {
        println!("{passed} passed, {failed} failed, {skipped} skipped");
    }
    exit
}
/// A parsed suite with its wired options.
type LoadedSuite = (TestFile, RunOptions);
/// Selection-indexed suite (reports reassemble in input order).
type IndexedSuite = (usize, TestFile, RunOptions);
/// Selection-indexed result.
type IndexedResult = (usize, SuiteResult);

/// Split parsed suites into (to-run, already-skipped), each tagged with its
/// selection index so reports reassemble in input order.
///
/// `skip: true` always skips. When any suite sets `focus: true`, only
/// focused suites run. A non-empty tag filter keeps suites carrying at
/// least one listed tag. Skipped suites report `skipped: true` (never fail).
fn select_suites(
    loaded: Vec<LoadedSuite>,
    tags: &[String],
) -> (Vec<IndexedSuite>, Vec<IndexedResult>) {
    let focused_any = loaded.iter().any(|(file, _)| file.focus);
    let mut run = Vec::new();
    let mut skipped = Vec::new();
    for (index, (file, opts)) in loaded.into_iter().enumerate() {
        let excluded = file.skip
            || (focused_any && !file.focus)
            || (!tags.is_empty() && !file.tags.iter().any(|tag| tags.contains(tag)));
        if excluded {
            skipped.push((index, skipped_result(&file.name)));
        } else {
            run.push((index, file, opts));
        }
    }
    (run, skipped)
}

/// Keep the Nth 1-based slice of M over the (already sorted) run list,
/// preserving selection indices.
fn apply_shard(run: Vec<IndexedSuite>, n: usize, m: usize) -> Vec<IndexedSuite> {
    run.into_iter()
        .enumerate()
        .filter(|(i, _)| i % m == n - 1)
        .map(|(_, pair)| pair)
        .collect()
}

/// A passing-but-skipped result so reports stay complete.
fn skipped_result(name: &str) -> SuiteResult {
    SuiteResult {
        schema: TestFile::schema_id().to_string(),
        suite: name.to_string(),
        passed: true,
        skipped: true,
        attempts: 1,
        exit_success: None,
        exit_signal: None,
        exit_code: None,
        duration_ms: 0,
        steps: Vec::new(),
        failure: None,
        terminal: TerminalInfo {
            width: 0,
            height: 0,
            term: String::new(),
        },
        trace: Vec::new(),
        trace_truncated: false,
        attachments: Vec::new(),
    }
}

/// Parse `N/M` shard selection (both 1-based, `N <= M`).
pub fn parse_shard(raw: &str) -> Result<(usize, usize), String> {
    let (n, m) = raw
        .split_once('/')
        .ok_or_else(|| format!("expected N/M, got {raw:?}"))?;
    let n: usize = n
        .parse()
        .map_err(|_| format!("bad shard index in {raw:?}"))?;
    let m: usize = m
        .parse()
        .map_err(|_| format!("bad shard total in {raw:?}"))?;
    if n == 0 || m == 0 || n > m {
        return Err(format!("shard needs 1 <= N <= M, got {raw:?}"));
    }
    Ok((n, m))
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
    let mark = if result.skipped {
        "○"
    } else if result.passed {
        "✓"
    } else {
        "✗"
    };
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

/// Filename-safe suite name (shared by traces + attachments).
fn safe_name(suite: &str) -> String {
    suite
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Copy suite-declared attachments into `reports/attachments/<suite>/`,
/// recording report-relative paths on each result. Paths resolve relative
/// to the invocation dir; missing files warn and skip (never fail the run).
fn collect_attachments(
    results: &mut [SuiteResult],
    declared: &std::collections::HashMap<String, Vec<String>>,
) {
    for result in results {
        if result.skipped {
            continue;
        }
        let Some(names) = declared.get(&result.suite) else {
            continue;
        };
        let safe = safe_name(&result.suite);
        for name in names {
            let src = Path::new(name);
            let Some(file_name) = src.file_name() else {
                eprintln!("warning: attachment has no file name: {name}");
                continue;
            };
            let dest = PathBuf::from("reports/attachments")
                .join(&safe)
                .join(file_name);
            if let Some(parent) = dest.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    eprintln!("warning: attachment dir {}: {e}", parent.display());
                    continue;
                }
            }
            match std::fs::copy(src, &dest) {
                Ok(_) => result.attachments.push(format!(
                    "attachments/{safe}/{}",
                    file_name.to_string_lossy()
                )),
                Err(e) => eprintln!("warning: attachment {name}: {e}"),
            }
        }
    }
}

/// Append one history line per suite to `reports/history.jsonl` (flake
/// tracking). Never fails the run.
fn append_history(results: &[SuiteResult]) {
    use std::io::Write as _;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let mut buf = String::new();
    for result in results {
        buf.push_str(
            &serde_json::json!({
                "ts": now,
                "suite": result.suite,
                "passed": result.passed,
                "skipped": result.skipped,
                "attempts": result.attempts,
                "duration_ms": result.duration_ms,
            })
            .to_string(),
        );
        buf.push('\n');
    }
    if let Err(e) = (|| -> std::io::Result<()> {
        let path = PathBuf::from("reports/history.jsonl");
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        file.write_all(buf.as_bytes())
    })() {
        eprintln!("warning: history append: {e}");
    }
}

/// Write `reports/traces/<suite>.zip` for failed suites — or every suite
/// with `--trace always` (`never` disables). Skipped suites have no run.
fn write_traces(results: &[SuiteResult], trace: Option<&str>) {
    if trace == Some("never") {
        return;
    }
    let always = trace == Some("always");
    for result in results {
        if result.skipped || (result.passed && !always) {
            continue;
        }
        let safe = safe_name(&result.suite);
        let path = PathBuf::from("reports/traces").join(format!("{safe}.zip"));
        match write_trace(&path, result) {
            Ok(()) => eprintln!("trace: {}", path.display()),
            Err(e) => eprintln!("warning: trace write {}: {e}", path.display()),
        }
    }
}

/// Render a trace timeline, or replay its raw bytes with original pacing.
pub fn trace(zip: &Path, replay: bool) -> i32 {
    let trace = match read_trace(zip) {
        Ok(trace) => trace,
        Err(e) => {
            eprintln!("read {}: {e}", zip.display());
            return EXIT_CONFIG_ERROR;
        }
    };
    if !replay {
        print!("{}", render_timeline(&trace));
        return EXIT_OK;
    }
    use std::io::Write as _;
    let mut out = std::io::stdout().lock();
    for (bytes, wait_ms) in replay_schedule(&trace, 1000) {
        if wait_ms > 0 {
            std::thread::sleep(std::time::Duration::from_millis(wait_ms));
        }
        if out.write_all(&bytes).is_err() {
            return EXIT_OK;
        }
    }
    let _ = out.flush();
    EXIT_OK
}

/// Re-render stored JSON results as JUnit or self-contained HTML.
pub fn report(format: &str, out: &Path, results_path: &Path) -> i32 {
    let results = match load_json_all(results_path) {
        Ok(results) => results,
        Err(e) => {
            eprintln!("read {}: {e}", results_path.display());
            return EXIT_CONFIG_ERROR;
        }
    };
    // Flake history lives next to the results file (B2); absent history
    // simply renders no flake section.
    let history_path = results_path
        .parent()
        .map(|parent| parent.join("history.jsonl"))
        .unwrap_or_else(|| PathBuf::from("history.jsonl"));
    let flakes =
        tui_lab_reporter::summarize_flakes(&tui_lab_reporter::load_history(&history_path), 30);
    let render: fn(&[tui_lab_core::SuiteResult], &[tui_lab_reporter::FlakeSummary]) -> String =
        match format {
            "junit" => |results, _| tui_lab_reporter::to_junit_all(results),
            "html" => tui_lab_reporter::to_html_with_flakes,
            _ => {
                eprintln!("--format {format}: expected junit or html");
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
    match std::fs::write(out, render(&results, &flakes)) {
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

    fn suite(name: &str, tags: &[&str], skip: bool, focus: bool) -> (TestFile, RunOptions) {
        let mut yaml = format!("schema: tui-lab/v1\nname: {name}\napplication:\n  command: x\n");
        if !tags.is_empty() {
            yaml.push_str("tags:\n");
            for tag in tags {
                yaml.push_str(&format!("  - {tag}\n"));
            }
        }
        yaml.push_str(&format!("skip: {skip}\nfocus: {focus}\nsteps: []\n"));
        let file = TestFile::from_yaml(&yaml).expect("parse");
        (file, RunOptions::default())
    }

    /// Phase A: shard parsing rejects nonsense.
    #[test]
    fn shard_syntax_is_strict() {
        assert_eq!(parse_shard("1/3"), Ok((1, 3)));
        assert_eq!(parse_shard("3/3"), Ok((3, 3)));
        assert!(parse_shard("0/3").is_err());
        assert!(parse_shard("4/3").is_err());
        assert!(parse_shard("1/0").is_err());
        assert!(parse_shard("all").is_err());
    }

    /// Phase A: skip/focus/tags partition suites; skipped never fail.
    #[test]
    fn selection_respects_skip_focus_tags() {
        let loaded = vec![
            suite("plain", &[], false, false),
            suite("tagged", &["smoke"], false, false),
            suite("skipped", &[], true, false),
        ];
        // No filter: skip only removes the skipped suite.
        let (run, skipped) = select_suites(loaded.clone(), &[]);
        assert_eq!(run.len(), 2);
        assert_eq!(skipped.len(), 1);
        assert!(skipped[0].1.skipped);
        assert!(skipped[0].1.passed, "skipped suites never fail");
        // Tag filter keeps matches only.
        let (run, skipped) = select_suites(loaded.clone(), &["smoke".to_string()]);
        assert_eq!(
            run.iter()
                .map(|(_, f, _)| f.name.as_str())
                .collect::<Vec<_>>(),
            ["tagged"]
        );
        assert_eq!(skipped.len(), 2);
        // Focus wins over everything except skip.
        let mut focused = loaded;
        focused[0].0.focus = true;
        let (run, _) = select_suites(focused, &[]);
        assert_eq!(
            run.iter()
                .map(|(_, f, _)| f.name.as_str())
                .collect::<Vec<_>>(),
            ["plain"]
        );
    }

    /// Phase A: sharding slices the sorted run list deterministically.
    #[test]
    fn shards_partition_without_overlap() {
        let loaded: Vec<LoadedSuite> = ["a", "b", "c"]
            .iter()
            .map(|name| suite(name, &[], false, false))
            .collect();
        let (run, _) = select_suites(loaded, &[]);
        let first = apply_shard(run.clone(), 1, 2);
        let second = apply_shard(run, 2, 2);
        let mut names: Vec<&str> = first
            .iter()
            .chain(second.iter())
            .map(|(_, f, _)| f.name.as_str())
            .collect();
        names.sort_unstable();
        assert_eq!(names, ["a", "b", "c"]);
        assert_eq!(first.len() + second.len(), 3);
        // Selection indices survive sharding (reports stay ordered).
        assert!(first.iter().chain(second.iter()).all(|(i, _, _)| *i < 3));
    }
}
