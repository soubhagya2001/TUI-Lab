//! Grid feeding: canned byte strings in, screen text/cursor out.
//!
//! Deterministic — no PTY, no processes, no timing.

use std::io::{self, Write};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use tui_lab_terminal::{Emulator, PtySink};

/// Writer that forwards everything into a channel for inspection.
struct ChanWriter {
    tx: mpsc::Sender<Vec<u8>>,
}

impl Write for ChanWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.tx
            .send(buf.to_vec())
            .map_err(|_| io::Error::other("receiver hung up"))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Emulator wired to a channel sink; returns it and the receiving end.
fn harness(cols: usize, rows: usize) -> (Emulator, mpsc::Receiver<Vec<u8>>) {
    let (tx, rx) = mpsc::channel();
    let sink: PtySink = Arc::new(Mutex::new(Box::new(ChanWriter { tx })));
    (Emulator::new(cols, rows, sink), rx)
}

#[test]
fn cursor_addressing_places_text() {
    let (mut emu, _) = harness(120, 40);
    emu.feed(b"Hello\x1b[2;5HWorld");
    let text = emu.text();
    let rows: Vec<&str> = text.lines().collect();
    assert_eq!(rows[0], "Hello");
    assert_eq!(&rows[1][4..9], "World");
    assert_eq!(emu.cursor(), (1, 9));
}

#[test]
fn clear_screen_empties_the_grid() {
    let (mut emu, _) = harness(80, 24);
    emu.feed(b"Hello");
    assert!(emu.text().contains("Hello"));
    emu.feed(b"\x1b[2J\x1b[H");
    assert!(!emu.text().contains("Hello"));
    assert_eq!(emu.cursor(), (0, 0));
}

#[test]
fn sgr_colors_do_not_leak_into_text() {
    let (mut emu, _) = harness(80, 24);
    emu.feed(b"\x1b[31mRed\x1b[0m Plain");
    let first = emu.text().lines().next().unwrap_or_default().to_string();
    assert_eq!(first, "Red Plain");
}

#[test]
fn alternate_screen_swap_is_stable() {
    let (mut emu, _) = harness(80, 24);
    emu.feed(b"primary\x1b[?1049h");
    emu.feed(b"alternate");
    assert!(emu.text().contains("alternate"));
    emu.feed(b"\x1b[?1049l");
    assert!(emu.text().contains("primary"));
}

#[test]
fn device_status_queries_are_answered_to_sink() {
    let (mut emu, rx) = harness(80, 24);
    emu.feed(b"\x1b[6n");
    let reply = rx
        .recv_timeout(Duration::from_secs(2))
        .expect("DSR reply written to sink");
    // Cursor-position report: ESC [ <row> ; <col> R
    assert!(
        reply.starts_with(b"\x1b[") && reply.ends_with(b"R"),
        "expected DSR reply, got {reply:?}"
    );
}

#[test]
fn resize_changes_dims_and_keeps_content_readable() {
    let (mut emu, _) = harness(120, 40);
    emu.feed(b"Hello");
    emu.resize(80, 24);
    assert_eq!(emu.dims(), (80, 24));
    assert!(emu.text().contains("Hello"));
}

#[test]
fn cells_carry_style_through_feed_and_read() {
    let (mut emu, _) = harness(80, 24);
    emu.feed(b"\x1b[31mR\x1b[0m\x1b[1mB\x1b[0mP");
    let cells = emu.cells();
    assert_eq!(cells.len(), 3);
    assert_eq!(cells[0].character, 'R');
    assert_eq!(cells[0].fg, "red");
    assert!(!cells[0].bold);
    assert_eq!(cells[1].character, 'B');
    assert!(cells[1].bold);
    assert_eq!(cells[2].character, 'P');
    assert!(!cells[2].bold);
}

#[test]
fn cells_skip_trailing_padding() {
    let (mut emu, _) = harness(80, 24);
    emu.feed(b"Hi");
    let cells = emu.cells();
    assert_eq!(cells.len(), 2);
    assert!(cells.iter().all(|cell| cell.y == 0));
}
