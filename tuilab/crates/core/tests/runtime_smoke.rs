//! Phase 1 exit gate: full runtime stack against the Ratatui fixture.
//!
//! Wires `tui-lab-pty` (spawn/pump) + `tui-lab-terminal` (grid, with DSR
//! forwarding back into the session writer) + `tui-lab-input` (keys):
//! launch → wait title → DOWN+ENTER → detail → resize → search filter → quit.
//! Mirrors the future `wait_for_text` + YAML flow without the protocol layer.

use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use tui_lab_input::{encode_key, encode_text};
use tui_lab_pty::{PtySession, SpawnOptions};
use tui_lab_terminal::Emulator;

const STEP: Duration = Duration::from_secs(10);

/// Build the fixture binary on demand; return its path.
fn fixture_bin() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/ratatui-sample");
    let output = Command::new("cargo")
        .arg("build")
        .current_dir(&dir)
        .output()
        .expect("run cargo build for fixture");
    assert!(
        output.status.success(),
        "fixture build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bin = if cfg!(windows) {
        "ratatui-sample.exe"
    } else {
        "ratatui-sample"
    };
    dir.join("target").join("debug").join(bin)
}

/// Poll the session, feed the grid live (handshake replies go back
/// immediately), and return once `needle` is visible.
fn wait_screen(sess: &PtySession, emu: &mut Emulator, needle: &str) -> bool {
    let start = Instant::now();
    while start.elapsed() < STEP {
        let chunk = sess.poll(Duration::from_millis(100));
        emu.feed(&chunk);
        if emu.text().contains(needle) {
            return true;
        }
    }
    false
}

fn press(sess: &mut PtySession, key: &str) {
    let bytes = encode_key(key).expect("encode key");
    sess.write_all(&bytes).expect("write key");
}

#[test]
fn runtime_smoke_against_ratatui_fixture() {
    let bin = fixture_bin();
    assert!(bin.is_file(), "fixture binary missing: {}", bin.display());

    let opts = SpawnOptions {
        command: bin.to_string_lossy().into_owned(),
        cols: 120,
        rows: 40,
        ..SpawnOptions::default()
    };
    let mut sess = PtySession::spawn(&opts).expect("spawn fixture");
    // The emulator writes handshake replies straight into the session.
    let mut emu = Emulator::new(120, 40, sess.writer_sink());

    // 1. Boot to the list screen.
    assert!(
        wait_screen(&sess, &mut emu, "TUI-LAB-SAMPLE"),
        "boot screen"
    );
    assert!(wait_screen(&sess, &mut emu, "alpha-table"), "list content");

    // 2. DOWN + ENTER selects the second item.
    press(&mut sess, "DOWN");
    press(&mut sess, "ENTER");
    assert!(
        wait_screen(&sess, &mut emu, "Selected: beta-chair"),
        "detail screen, got:\n{}",
        emu.text()
    );

    // 3. Resize: session and grid shrink together, app keeps rendering.
    sess.resize(80, 24).expect("resize pty");
    emu.resize(80, 24);
    assert_eq!(emu.dims(), (80, 24));
    assert!(
        wait_screen(&sess, &mut emu, "Selected: beta-chair"),
        "detail survives resize, got:\n{}",
        emu.text()
    );

    // 4. Back to list, search-filter for "table".
    press(&mut sess, "ESC");
    assert!(
        wait_screen(&sess, &mut emu, "TUI-LAB-SAMPLE"),
        "back to list"
    );
    press(&mut sess, "/");
    sess.write_all(&encode_text("table")).expect("type query");
    assert!(
        wait_screen(&sess, &mut emu, "Search: table"),
        "search input, got:\n{}",
        emu.text()
    );
    assert!(emu.text().contains("gamma-table"), "filter keeps matches");
    assert!(
        !emu.text().contains("beta-chair"),
        "filter hides non-matches, got:\n{}",
        emu.text()
    );

    // 5. Clean quit from search (ESC) then list (q), exit code 0.
    press(&mut sess, "ESC");
    assert!(
        wait_screen(&sess, &mut emu, "TUI-LAB-SAMPLE"),
        "exit search"
    );
    let status = sess.close(Some(b"q"), None).expect("bounded close");
    assert!(status.success(), "fixture exits 0, got {status:?}");
}
