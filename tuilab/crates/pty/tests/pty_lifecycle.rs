//! PTY lifecycle: spawn, echo round-trip, resize, bounded close.
//!
//! Runs on Windows (ConPTY + `cmd`) and Unix (`cat`). No grid involved —
//! that belongs to `tui-lab-terminal`.
//!
//! Windows note (docs/12 §12.4): ConPTY opens with `ESC[6n` and stalls until
//! the consumer answers with a cursor-position report. The real answer path
//! lives in `tui-lab-terminal` (`Event::PtyWrite` forwarding); this test
//! answers minimally inline so the raw pipe round-trip can be proven here.

use std::time::{Duration, Instant};

use tui_lab_pty::{PtySession, SpawnOptions};

/// ConPTY cursor-position request.
const DSR_QUERY: &[u8] = b"\x1b[6n";
/// Minimal report: row 1, col 1.
const DSR_REPLY: &[u8] = b"\x1b[1;1R";

#[cfg(windows)]
fn shell_options() -> (SpawnOptions, Vec<u8>, Option<&'static [u8]>) {
    (
        SpawnOptions {
            command: "cmd".to_string(),
            args: vec!["/Q".to_string()],
            ..SpawnOptions::default()
        },
        // `echo` keeps cmd's errorlevel at 0 so the graceful exit asserts cleanly.
        b"echo PTY_ALIVE_789\r".to_vec(),
        Some(b"exit\r"),
    )
}

#[cfg(not(windows))]
fn shell_options() -> (SpawnOptions, Vec<u8>, Option<&'static [u8]>) {
    (
        SpawnOptions {
            command: "cat".to_string(),
            ..SpawnOptions::default()
        },
        b"PTY_ALIVE_789\n".to_vec(),
        None,
    )
}

/// Poll for `needle`, answering DSR queries so ConPTY handshakes complete.
fn poll_answering(sess: &mut PtySession, needle: &str, deadline: Duration) -> Option<Vec<u8>> {
    let start = Instant::now();
    let mut out = Vec::new();
    while start.elapsed() < deadline {
        let chunk = sess.poll(Duration::from_millis(200));
        if chunk.windows(DSR_QUERY.len()).any(|w| w == DSR_QUERY) {
            sess.write_all(DSR_REPLY).expect("answer DSR");
        }
        out.extend_from_slice(&chunk);
        if String::from_utf8_lossy(&out).contains(needle) {
            return Some(out);
        }
    }
    None
}

#[test]
fn echo_round_trip_then_bounded_close() {
    let (opts, probe, quit) = shell_options();
    let mut sess = PtySession::spawn(&opts).expect("spawn shell");

    // Unix `cat` has no prompt; Windows `cmd` prints one after the handshake.
    #[cfg(windows)]
    poll_answering(&mut sess, ">", Duration::from_secs(10)).expect("shell prompt");

    sess.write_all(&probe).expect("write probe");
    let out = poll_answering(&mut sess, "PTY_ALIVE_789", Duration::from_secs(10))
        .expect("echo round-trip within deadline");

    // The marker must come back through the PTY output pump.
    assert!(String::from_utf8_lossy(&out).contains("PTY_ALIVE_789"));

    // The pump delivered bytes with a clean read record.
    let (bytes, errors) = sess.pump_stats();
    assert!(bytes > 0, "pump delivered output");
    assert_eq!(errors, 0, "no read errors during round-trip");

    // Resize must not kill the session: a second round-trip still works.
    sess.resize(80, 24).expect("resize PTY");
    sess.write_all(&probe).expect("write after resize");
    poll_answering(&mut sess, "PTY_ALIVE_789", Duration::from_secs(10))
        .expect("session survives resize");

    // Bounded close: graceful quit where possible, kill otherwise.
    let status = sess.close(quit, None).expect("bounded close");
    if quit.is_some() {
        assert!(status.success(), "graceful quit exits 0, got {status:?}");
    }
}
