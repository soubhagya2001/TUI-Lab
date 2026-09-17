//! `wait_for_text`: poll a screen closure until text appears (docs/06 §6.4).
//!
//! Generic over the screen source so tests feed canned screens without a PTY.
//! Timeouts return the last screen + elapsed time — the failure-bundle
//! payload specified in docs/11 §11.3.

use std::time::{Duration, Instant};

/// Outcome of a wait: found or timed out, always with evidence.
pub struct WaitOutcome {
    /// Whether the needle appeared in time.
    pub found: bool,
    /// Wall time spent waiting.
    pub elapsed: Duration,
    /// Last screen observed (for failure bundles).
    pub last_screen: String,
}

/// Substring or regex match of screen text.
#[must_use]
pub fn matches(screen: &str, needle: &str, regex: bool) -> bool {
    if regex {
        regex::Regex::new(needle).is_ok_and(|pattern| pattern.is_match(screen))
    } else {
        screen.contains(needle)
    }
}

/// Poll `screen()` every `poll` until it matches or `timeout` elapses.
pub async fn wait_for_text<F>(
    mut screen: F,
    needle: &str,
    regex: bool,
    timeout: Duration,
    poll: Duration,
) -> WaitOutcome
where
    F: FnMut() -> String,
{
    let start = Instant::now();
    let mut last_screen = String::new();
    while start.elapsed() < timeout {
        last_screen = screen();
        if matches(&last_screen, needle, regex) {
            return WaitOutcome {
                found: true,
                elapsed: start.elapsed(),
                last_screen,
            };
        }
        tokio::time::sleep(poll).await;
    }
    WaitOutcome {
        found: false,
        elapsed: start.elapsed(),
        last_screen,
    }
}
