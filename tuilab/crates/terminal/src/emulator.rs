//! Grid emulator adapter over `alacritty_terminal` (docs/03 §3.2).
//!
//! The grid/state machine is reused, not hand-rolled. Two integration rules
//! from the Phase 0 spike (docs/12 §12.4):
//! * every received chunk is pumped through the parser **immediately**, so
//!   handshake responses go back without delay;
//! * terminal-initiated writes (`Event::PtyWrite`: DSR answers, DA replies)
//!   are forwarded to the PTY master writer — dropping them stalls ConPTY.

use std::io::Write;
use std::sync::{Arc, Mutex};

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::Dimensions as _;
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::term::test::TermSize;
use alacritty_terminal::term::Term;
use alacritty_terminal::vte::ansi::{Processor, StdSyncHandler};

use crate::utils::render_text;

/// Shared PTY writer. `tui-lab-pty` hands its writer over in Phase 2;
/// tests substitute a swallowing buffer.
pub type PtySink = Arc<Mutex<Box<dyn Write + Send>>>;

/// Forwards `Event::PtyWrite` into the PTY master.
#[derive(Clone)]
pub struct ForwardingListener {
    sink: PtySink,
}

impl ForwardingListener {
    /// Build a listener writing back into `sink`.
    pub fn new(sink: PtySink) -> Self {
        Self { sink }
    }
}

impl EventListener for ForwardingListener {
    fn send_event(&self, event: Event) {
        if let Event::PtyWrite(text) = event {
            if let Ok(mut sink) = self.sink.lock() {
                let _ = sink.write_all(text.as_bytes());
            }
        }
    }
}

/// A virtual terminal screen fed with raw PTY bytes.
pub struct Emulator {
    term: Term<ForwardingListener>,
    processor: Processor<StdSyncHandler>,
}

impl Emulator {
    /// Create a `cols` x `rows` screen writing back into `sink`.
    pub fn new(cols: usize, rows: usize, sink: PtySink) -> Self {
        let listener = ForwardingListener::new(sink);
        Self {
            term: Term::new(Default::default(), &TermSize::new(cols, rows), listener),
            processor: Processor::<StdSyncHandler>::new(),
        }
    }

    /// Feed raw PTY output bytes into the grid.
    pub fn feed(&mut self, bytes: &[u8]) {
        self.processor.advance(&mut self.term, bytes);
    }

    /// Plain-text dump of the visible grid (trailing space trimmed per row).
    pub fn text(&self) -> String {
        let grid = self.term.grid();
        let rows: Vec<Vec<char>> = (0..self.term.screen_lines())
            .map(|line| {
                (0..self.term.columns())
                    .map(|col| grid[Line(line as i32)][Column(col)].c)
                    .collect()
            })
            .collect();
        render_text(&rows)
    }

    /// Zero-based visible `(row, col)` of the cursor.
    pub fn cursor(&self) -> (usize, usize) {
        let point = self.term.grid().cursor.point;
        (point.line.0.max(0) as usize, point.column.0)
    }

    /// Visible `(cols, rows)`.
    pub fn dims(&self) -> (usize, usize) {
        (self.term.columns(), self.term.screen_lines())
    }

    /// Resize the grid.
    pub fn resize(&mut self, cols: usize, rows: usize) {
        self.term.resize(TermSize::new(cols, rows));
    }
}
