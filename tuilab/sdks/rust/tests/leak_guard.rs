//! K4: dropping a session without `close` must not leak the sidecar.
//!
//! The leak path is the one tests never hit: every other test closes
//! cleanly, so a missing drop guard ships unnoticed. These pin the abrupt
//! path — the sidecar process must actually die.

use std::time::Duration;

use tui_lab_sdk::{Connection, LaunchOptions, TuiTest};

/// Any long-lived child works — this test only watches process death.
#[cfg(windows)]
const SHELL: &str = "cmd";
#[cfg(not(windows))]
const SHELL: &str = "sh";

/// Whether `pid` names a live process. Unknown-tool failures answer `true`
/// (never fail a suite because a helper binary is missing).
fn pid_alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        let Ok(out) = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
            .output()
        else {
            return true;
        };
        // Match the quoted pid field so a localized "no tasks" banner
        // (different language, still exit 0) can't be mistaken for a hit.
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .any(|line| line.contains(&format!("\"{pid}\"")))
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new("sh")
            .args(["-c", &format!("kill -0 {pid}")])
            .status()
            .map(|status| status.success())
            .unwrap_or(true)
    }
}

async fn wait_until_gone(pid: u32) -> bool {
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    while std::time::Instant::now() < deadline {
        if !pid_alive(pid) {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    !pid_alive(pid)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dropped_connection_kills_the_sidecar() {
    let conn = Connection::spawn(None).await.expect("spawn sidecar");
    let pid = conn.pid().expect("sidecar pid");
    assert!(pid_alive(pid), "sidecar runs before the drop");
    drop(conn);
    assert!(
        wait_until_gone(pid).await,
        "dropped sidecar {pid} must die (K4)"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dropped_session_takes_the_sidecar_with_it() {
    let tui = TuiTest::launch(SHELL, LaunchOptions::new())
        .await
        .expect("launch");
    let pid = tui.sidecar_pid().expect("sidecar pid");
    assert!(pid_alive(pid));
    // No `close` — the leak path a forgotten guard would leave behind.
    drop(tui);
    assert!(
        wait_until_gone(pid).await,
        "dropped session must not leave sidecar {pid} running (K4)"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn explicit_close_is_unaffected() {
    let conn = Connection::spawn(None).await.expect("spawn sidecar");
    let pid = conn.pid().expect("sidecar pid");
    conn.close().await.expect("close");
    assert!(
        wait_until_gone(pid).await,
        "explicit close reaps the sidecar"
    );
}
