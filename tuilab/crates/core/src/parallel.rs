//! Parallel suite fan-out: one session per suite, ordered results (Phase 8).
//!
//! Each suite runs through the untouched single-session [`run_file`], so no
//! shared mutable state exists between tasks. A semaphore bounds concurrent
//! PTYs at `min(max_parallel, registry cap)`; results reassemble in input
//! order regardless of completion order (diffable reports). A failing suite
//! never aborts its siblings — failures are data, matching the sequential
//! runner (run-all/report-all).

use std::sync::Arc;

use tokio::sync::Semaphore;
use tui_lab_protocol::TestFile;

use crate::result::{FailureInfo, SuiteResult, TerminalInfo};
use crate::runner::{run_file, RunOptions};
use crate::sessions::DEFAULT_MAX_SESSIONS;

/// Run parsed suites concurrently; returned in input order.
///
/// `max_parallel` clamps to `[1, registry cap]`. Infrastructure errors become
/// failed results (named suite, error in `failure`) instead of aborting.
pub async fn run_suites(
    files: Vec<TestFile>,
    opts: &RunOptions,
    max_parallel: usize,
) -> Vec<SuiteResult> {
    let slots = max_parallel.clamp(1, DEFAULT_MAX_SESSIONS);
    let semaphore = Arc::new(Semaphore::new(slots));
    let mut set = tokio::task::JoinSet::new();

    for (index, file) in files.into_iter().enumerate() {
        let permit_source = Arc::clone(&semaphore);
        let opts = opts.clone();
        set.spawn(async move {
            let _permit = permit_source.acquire_owned().await.expect("semaphore open");
            let name = file.name.clone();
            let outcome = run_file(&file, &opts).await;
            (index, name, outcome)
        });
    }

    let mut ordered: Vec<(usize, SuiteResult)> = Vec::new();
    while let Some(joined) = set.join_next().await {
        match joined {
            Ok((index, name, Ok(result))) => {
                debug_assert_eq!(result.suite, name);
                ordered.push((index, result));
            }
            Ok((index, name, Err(error))) => {
                ordered.push((index, infra_failure(name, &error.to_string())))
            }
            Err(error) => ordered.push((
                usize::MAX,
                infra_failure("<join>".to_string(), &format!("worker failed: {error}")),
            )),
        }
    }
    ordered.sort_by_key(|(index, _)| *index);
    ordered.into_iter().map(|(_, result)| result).collect()
}

/// A failed result for infrastructure breakdowns (launch/PTY/timeout).
fn infra_failure(suite: String, error: &str) -> SuiteResult {
    SuiteResult {
        schema: tui_lab_protocol::TestFile::schema_id().to_string(),
        suite: suite.clone(),
        passed: false,
        exit_success: None,
        exit_signal: None,
        exit_code: None,
        duration_ms: 0,
        steps: Vec::new(),
        failure: Some(FailureInfo {
            step_index: 0,
            step: "launch".to_string(),
            expected: format!("suite {suite:?} runs"),
            actual: error.to_string(),
            last_screen: String::new(),
            input_history: Vec::new(),
        }),
        terminal: TerminalInfo {
            width: 0,
            height: 0,
            term: String::new(),
        },
    }
}
