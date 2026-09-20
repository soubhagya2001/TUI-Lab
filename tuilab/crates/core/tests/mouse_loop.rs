//! Mouse full loop: SGR bytes in, app readout out (Phase 9b).
//!
//! Drives the named press surface (`CLICK x y`, …) against the live
//! fixture — proving a real app receives the events, not just byte shapes.
//! SGR coordinates are 1-based; crossterm reports 0-based.

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use tui_lab_input::encode_key;
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

/// Pump until `needle` is visible; return the screen or panic with it.
fn wait_screen(sess: &PtySession, emu: &mut Emulator, needle: &str) -> String {
    let start = std::time::Instant::now();
    loop {
        let chunk = sess.poll(Duration::from_millis(100));
        emu.feed(&chunk);
        let text = emu.text();
        if text.contains(needle) {
            return text;
        }
        assert!(
            start.elapsed() < STEP,
            "wait_for {needle:?} timed out, last screen:\n{text}"
        );
    }
}

fn press(sess: &mut PtySession, key: &str) {
    let bytes = encode_key(key).expect("encode mouse key");
    sess.write_all(&bytes).expect("write key");
}

#[test]
fn click_drag_and_scroll_reach_the_app() {
    let bin = fixture_bin();
    let mut sess = PtySession::spawn(&SpawnOptions {
        command: bin.to_string_lossy().into_owned(),
        cols: 120,
        rows: 40,
        ..SpawnOptions::default()
    })
    .expect("spawn fixture");
    let mut emu = Emulator::new(120, 40, sess.writer_sink());

    wait_screen(&sess, &mut emu, "TUI-LAB-SAMPLE");
    wait_screen(&sess, &mut emu, "Mouse: -");

    // Click (SGR press): crossterm reports 0-based coords.
    press(&mut sess, "CLICK 10 2");
    wait_screen(&sess, &mut emu, "Mouse: Down(Left) 9,1");

    // Wheel-up tick at the same cell.
    press(&mut sess, "SCROLL_UP 10 2");
    wait_screen(&sess, &mut emu, "Mouse: ScrollUp 9,1");

    // Drag end as its own step: button-coded release (Cb=0+m), because
    // ConPTY does not translate the generic Cb=3 release into an input
    // record (proven live — see docs/03). Gestures always travel as
    // distinct steps, exactly like real event streams.
    press(&mut sess, "RELEASE 30 2");
    wait_screen(&sess, &mut emu, "Mouse: Up(Left) 29,1");

    // Clean quit like every other suite.
    press(&mut sess, "q");
    let status = sess.close(None, None).expect("bounded close");
    assert!(status.success(), "fixture exits 0, got {status:?}");
}
