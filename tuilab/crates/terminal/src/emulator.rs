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
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::test::TermSize;
use alacritty_terminal::term::Term;
use alacritty_terminal::vte::ansi::{Color, Processor, StdSyncHandler};

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
                // R7: a failed handshake reply stalls ConPTY silently —
                // trace it instead of swallowing.
                if let Err(e) = sink.write_all(text.as_bytes()) {
                    tracing::warn!("PtyWrite forward failed: {e}");
                }
            }
        }
    }
}

/// One grid cell with style, grid-owned (docs/06 §6.2 render snapshot).
///
/// `tui-lab-snapshots` maps this onto its `CellData`; layers stay clean.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyledCell {
    /// Zero-based column.
    pub x: usize,
    /// Zero-based row.
    pub y: usize,
    /// Grapheme.
    pub character: char,
    /// Foreground (`black`, `brightred`, `#rrggbb`, `color{n}`, …).
    pub fg: String,
    /// Background, same encoding.
    pub bg: String,
    /// Bold flag.
    pub bold: bool,
    /// Any underline variant.
    pub underline: bool,
    /// Reverse video.
    pub reverse: bool,
}

/// Deterministic color encoding for snapshots and assertions.
fn color_name(color: &Color) -> String {
    match color {
        Color::Named(named) => format!("{named:?}").to_lowercase(),
        Color::Spec(rgb) => format!("#{:02x}{:02x}{:02x}", rgb.r, rgb.g, rgb.b),
        Color::Indexed(index) => format!("color{index}"),
    }
}

/// A virtual terminal screen fed with raw PTY bytes.
pub struct Emulator {
    term: Term<ForwardingListener>,
    processor: Processor<StdSyncHandler>,
    /// Last OSC 52 clipboard payload (base64, as sent).
    clipboard: Option<String>,
    /// Partial OSC sequence carried across `feed` calls.
    osc_carry: Vec<u8>,
    /// Captured Sixel payloads (normalized, in arrival order).
    sixels: Vec<Vec<u8>>,
    /// Partial DCS sequence carried across `feed` calls.
    dcs_carry: Vec<u8>,
}

impl Emulator {
    /// Create a `cols` x `rows` screen writing back into `sink`.
    pub fn new(cols: usize, rows: usize, sink: PtySink) -> Self {
        let listener = ForwardingListener::new(sink);
        Self {
            term: Term::new(Default::default(), &TermSize::new(cols, rows), listener),
            processor: Processor::<StdSyncHandler>::new(),
            clipboard: None,
            osc_carry: Vec::new(),
            sixels: Vec::new(),
            dcs_carry: Vec::new(),
        }
    }

    /// Feed raw PTY output bytes into the grid.
    pub fn feed(&mut self, bytes: &[u8]) {
        self.scan_osc52(bytes);
        self.scan_sixel(bytes);
        self.processor.advance(&mut self.term, bytes);
    }

    /// Last OSC 52 clipboard payload (base64, as sent), if the app set one.
    pub fn clipboard(&self) -> Option<&str> {
        self.clipboard.as_deref()
    }
    /// Scrollback history lines, oldest first (plain text per row).
    pub fn scrollback_lines(&self) -> Vec<String> {
        let grid = self.term.grid();
        let history = grid.history_size();
        let cols = self.term.columns();
        (0..history)
            .map(|i| {
                let line = Line(-(history as i32) + i as i32);
                (0..cols)
                    .map(|col| grid[line][Column(col)].c)
                    .collect::<String>()
            })
            .collect()
    }

    /// Search scrollback (oldest first); returns the history row index.
    pub fn find_scrollback(&self, needle: &str) -> Option<usize> {
        self.scrollback_lines()
            .iter()
            .position(|line| line.contains(needle))
    }

    /// Sniff OSC 52 clipboard sets (`ESC ] 52 ; <sel> ; <base64> BEL|ESC\`).
    ///
    /// Bytes always pass through to the grid untouched; only the payload is
    /// recorded. Partial sequences carry to the next feed (bounded); junk
    /// beyond the cap is dropped.
    fn scan_osc52(&mut self, bytes: &[u8]) {
        const PREFIX: &[u8] = b"\x1b]52;";
        const CAP: usize = 4096;
        self.osc_carry.extend_from_slice(bytes);
        if self.osc_carry.len() > CAP + 64 {
            self.osc_carry.drain(..self.osc_carry.len() - CAP);
        }
        // Process all complete sequences; hold a trailing partial one.
        loop {
            let start = match self
                .osc_carry
                .windows(PREFIX.len())
                .position(|w| w == PREFIX)
            {
                Some(pos) => pos,
                None => {
                    // Keep a trailing partial prefix, drop the rest.
                    let keep = PREFIX.len() - 1;
                    let len = self.osc_carry.len();
                    self.osc_carry.drain(..len.saturating_sub(keep));
                    return;
                }
            };
            let rest = &self.osc_carry[start + PREFIX.len()..];
            let end = rest
                .iter()
                .position(|&b| b == 0x07)
                .map(|p| (p, 1))
                .or_else(|| rest.windows(2).position(|w| w == b"\x1b\\").map(|p| (p, 2)));
            let Some((rel, term_len)) = end else {
                // Partial: hold from the prefix on.
                self.osc_carry.drain(..start);
                return;
            };
            let body = &rest[..rel];
            // Body is `<sel>;<base64>` — selection ignored, payload kept.
            if let Some(semi) = body.iter().position(|&b| b == b';') {
                if let Ok(payload) = std::str::from_utf8(&body[semi + 1..]) {
                    self.clipboard = Some(payload.to_string());
                }
            }
            let drop_upto = start + PREFIX.len() + rel + term_len;
            self.osc_carry.drain(..drop_upto);
        }
    }

    /// Captured Sixel payloads, normalized, in arrival order.
    pub fn sixels(&self) -> &[Vec<u8>] {
        &self.sixels
    }

    /// Sniff Sixel graphics (`ESC P … q … ESC\` DCS sequences).
    ///
    /// Only DCS blocks whose parameter section introduces Sixel (`q`) are
    /// kept; other device-control strings (e.g. XTGETTCAP replies) pass
    /// through ignored. Payloads normalize by stripping whitespace so
    /// equivalent encodings compare equal. Partial blocks carry to the next
    /// feed (bounded); the grid itself never sees graphics (the grid crate
    /// has no Sixel support — this is structural capture, not rendering).
    fn scan_sixel(&mut self, bytes: &[u8]) {
        const PREFIX: &[u8] = b"\x1bP";
        const CAP: usize = 1024 * 1024;
        self.dcs_carry.extend_from_slice(bytes);
        if self.dcs_carry.len() > CAP + 64 {
            self.dcs_carry.drain(..self.dcs_carry.len() - CAP);
        }
        loop {
            let start = match self
                .dcs_carry
                .windows(PREFIX.len())
                .position(|w| w == PREFIX)
            {
                Some(pos) => pos,
                None => {
                    let len = self.dcs_carry.len();
                    self.dcs_carry.drain(..len.saturating_sub(PREFIX.len() - 1));
                    return;
                }
            };
            let rest = &self.dcs_carry[start + PREFIX.len()..];
            let end = rest
                .windows(2)
                .position(|w| w == b"\x1b\\")
                .map(|p| (p, 2))
                .or_else(|| rest.iter().position(|&b| b == 0x07).map(|p| (p, 1)));
            let Some((rel, term_len)) = end else {
                self.dcs_carry.drain(..start);
                return;
            };
            let body = &rest[..rel];
            // Sixel introducer: optional digit/; params, then `q`.
            let params_end = body
                .iter()
                .position(|&b| !(b.is_ascii_digit() || b == b';'))
                .unwrap_or(body.len());
            if body.get(params_end) != Some(&b'q') {
                // Not Sixel (e.g. XTGETTCAP reply) — drop through terminator.
                self.dcs_carry
                    .drain(..start + PREFIX.len() + rel + term_len);
                continue;
            }
            let payload: Vec<u8> = body[params_end + 1..]
                .iter()
                .copied()
                .filter(|&b| !b.is_ascii_whitespace())
                .collect();
            self.sixels.push(payload);
            self.dcs_carry
                .drain(..start + PREFIX.len() + rel + term_len);
        }
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

    /// Heuristic accessibility tree for agents and `role` assertions.
    pub fn a11y_tree(&self) -> Vec<crate::a11y::A11yNode> {
        let rows: Vec<String> = self.text().lines().map(str::to_string).collect();
        crate::a11y::build_tree(&rows, self.cursor())
    }

    /// Visible `(cols, rows)`.
    pub fn dims(&self) -> (usize, usize) {
        (self.term.columns(), self.term.screen_lines())
    }

    /// Cell-scoped text for `assert_region` (R3).
    ///
    /// Coordinates are display cells (columns/rows), matching the geometry
    /// users see. Wide characters occupy two cells; the spacer cell is
    /// skipped so `"日本"` in 4 columns reads as two chars, not four.
    /// Out-of-bounds regions are an error naming the bounds (never a
    /// misleading empty mismatch).
    pub fn region_text(
        &self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
    ) -> Result<String, String> {
        let (cols, rows) = self.dims();
        if x + width > cols || y + height > rows {
            return Err(format!(
                "region ({x},{y}) {width}x{height} outside {cols}x{rows}"
            ));
        }
        let grid = self.term.grid();
        let mut lines = Vec::with_capacity(height);
        for row in y..y + height {
            let mut line = String::new();
            for col in x..x + width {
                let cell = &grid[Line(row as i32)][Column(col)];
                if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                    continue;
                }
                line.push(cell.c);
            }
            lines.push(line);
        }
        Ok(lines.join("\n"))
    }

    /// Styled grid dump, row-major (excludes trailing blank padding per row
    /// like [`Emulator::text`], so snapshots stay compact).
    pub fn cells(&self) -> Vec<StyledCell> {
        let grid = self.term.grid();
        let mut out = Vec::new();
        for row in 0..self.term.screen_lines() {
            let mut last_content = 0;
            for col in 0..self.term.columns() {
                if grid[Line(row as i32)][Column(col)].c != ' ' {
                    last_content = col + 1;
                }
            }
            for col in 0..last_content {
                let cell = &grid[Line(row as i32)][Column(col)];
                out.push(StyledCell {
                    x: col,
                    y: row,
                    character: cell.c,
                    fg: color_name(&cell.fg),
                    bg: color_name(&cell.bg),
                    bold: cell.flags.contains(Flags::BOLD),
                    underline: cell.flags.intersects(Flags::ALL_UNDERLINES),
                    reverse: cell.flags.contains(Flags::INVERSE),
                });
            }
        }
        out
    }

    /// Resize the grid.
    pub fn resize(&mut self, cols: usize, rows: usize) {
        self.term.resize(TermSize::new(cols, rows));
    }
}
