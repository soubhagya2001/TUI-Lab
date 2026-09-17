//! `wait_for_text` behavior with canned screens — no PTY, no timing flakes.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tui_lab_runtime::{matches, run_with_timeout, wait_for_text};

#[test]
fn matcher_handles_substring_and_regex() {
    assert!(matches("Welcome to Dashboard", "Dashboard", false));
    assert!(!matches("Welcome", "Dashboard", false));
    assert!(matches("CPU: 20%", r"CPU: \d+%", true));
    assert!(!matches("CPU: high", r"CPU: \d+%", true));
    // Invalid regex never matches (and never panics).
    assert!(!matches("anything", "([", true));
}

#[tokio::test]
async fn immediate_match_returns_fast() {
    let outcome = wait_for_text(
        || "Dashboard ready".to_string(),
        "Dashboard",
        false,
        Duration::from_secs(5),
        Duration::from_millis(10),
    )
    .await;
    assert!(outcome.found);
    assert!(outcome.elapsed < Duration::from_secs(1));
    assert!(outcome.last_screen.contains("Dashboard"));
}

#[tokio::test]
async fn late_text_is_found() {
    let screen = Arc::new(Mutex::new("booting…".to_string()));
    let writer = Arc::clone(&screen);
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        *writer.lock().expect("screen lock") = "Dashboard ready".to_string();
    });
    let outcome = wait_for_text(
        || screen.lock().expect("screen lock").clone(),
        "Dashboard",
        false,
        Duration::from_secs(5),
        Duration::from_millis(10),
    )
    .await;
    assert!(outcome.found);
    assert!(outcome.last_screen.contains("Dashboard"));
}

#[tokio::test]
async fn missing_text_times_out_with_evidence() {
    let outcome = wait_for_text(
        || "nothing here".to_string(),
        "Dashboard",
        false,
        Duration::from_millis(150),
        Duration::from_millis(10),
    )
    .await;
    assert!(!outcome.found);
    assert!(outcome.elapsed >= Duration::from_millis(150));
    // Failure-bundle payload: the last screen observed.
    assert_eq!(outcome.last_screen, "nothing here");
}

#[tokio::test]
async fn supervisor_bounds_futures() {
    let fast = run_with_timeout(async { 42 }, Duration::from_secs(1), "fast").await;
    assert_eq!(fast.expect("fast completes"), 42);

    let slow = run_with_timeout(
        async {
            tokio::time::sleep(Duration::from_secs(30)).await;
        },
        Duration::from_millis(50),
        "slow",
    )
    .await;
    assert!(slow.is_err(), "slow future must time out");
}
