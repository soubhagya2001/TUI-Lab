//! Interactive recorder: drive the app, get YAML back (`docs/10`).
//!
//! `tuilab record` owns the PTY: keystrokes from our stdin go to the app,
//! the app screen renders on our stdout, and every input is logged. Beats
//! are paced (cap + stabilize gate) so smart waits synthesize faithfully:
//! after each beat the screen must settle before the next beat is consumed,
//! and each settled screen becomes a `wait_for_text` for the beat that
//! produced it. Sleeps are never emitted — waits are (`AGENTS.md` §3).
//!
//! End the session with Ctrl+\ (not forwarded) or stdin EOF. Quit the app
//! first (e.g. `q`); the emitted suite asserts `exit_code: 0`.

use std::io::{IsTerminal as _, Read, Write};
use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use tui_lab_core::{NewSession, SessionRegistry};
use tui_lab_input::{decode_key, Key};
use tui_lab_protocol::{Step, SuiteAssertion, TestFile, WaitForText};
use tui_lab_pty::SpawnOptions;

use crate::constants::{
    EXIT_CONFIG_ERROR, EXIT_OK, REC_BEAT_WAIT_MS, REC_CHUNK_BYTES, REC_FINISH_BYTE,
    REC_STABILIZE_TIMEOUT_MS, REC_STABLE_POLLS, REC_TICK_MS,
};

/// Best-effort raw mode (unavailable on pipes — recording still works).
struct RawGuard {
    active: bool,
}

impl RawGuard {
    fn acquire() -> Self {
        match crossterm::terminal::enable_raw_mode() {
            Ok(()) => Self { active: true },
            Err(e) => {
                eprintln!("warning: raw mode unavailable ({e}); continuing");
                Self { active: false }
            }
        }
    }
}

impl Drop for RawGuard {
    fn drop(&mut self) {
        if self.active {
            let _ = crossterm::terminal::disable_raw_mode();
        }
    }
}

/// Record `--command` to a YAML suite at `out`.
pub fn run(command: &str, args: &[String], out: &Path, width: u16, height: u16) -> i32 {
    let mut registry = SessionRegistry::new();
    let id = match registry.spawn(NewSession::new(SpawnOptions {
        command: command.to_string(),
        args: args.to_vec(),
        cwd: None,
        env: Vec::new(),
        cols: width,
        rows: height,
    })) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("record: launch failed: {e}");
            return EXIT_CONFIG_ERROR;
        }
    };

    let _raw = RawGuard::acquire();
    let interactive = std::io::stdout().is_terminal();

    // Stdin pump thread: blocking reads stay off the record loop.
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        let mut stdin = std::io::stdin().lock();
        let mut chunk = [0u8; 1024];
        loop {
            match stdin.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    if tx.send(chunk[..n].to_vec()).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let mut steps: Vec<Step> = Vec::new();
    let mut pending: Vec<Step> = Vec::new();
    let mut type_run = String::new();
    let mut carry: Vec<u8> = Vec::new();
    let mut dropped_bytes: usize = 0;

    // Initial beat: wait for first content (slow boots), settle, and emit
    // the boot wait every suite starts with.
    drain_until_changed(&mut registry, &id, interactive, "", Duration::from_secs(10));
    let mut last_stable = stabilize(&mut registry, &id, interactive);
    emit_wait_for(&mut steps, &[], &last_stable);

    let mut finished = false;
    while !finished {
        // Top up: block briefly for fresh bytes, then drain what's buffered.
        match rx.recv_timeout(Duration::from_millis(REC_BEAT_WAIT_MS)) {
            Ok(chunk) => carry.extend_from_slice(&chunk),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => finished = true,
        }
        while carry.len() < REC_CHUNK_BYTES && !finished {
            match rx.try_recv() {
                Ok(chunk) => carry.extend_from_slice(&chunk),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    finished = true;
                    break;
                }
            }
        }
        if carry.is_empty() {
            if finished {
                break;
            }
            // Idle tick: keep the preview alive.
            render(&mut registry, &id, interactive);
            continue;
        }
        // One beat: at most CHUNK bytes; the tail (including incomplete key
        // sequences) stays buffered for the next beat.
        let take = carry.len().min(REC_CHUNK_BYTES);
        let mut beat: Vec<u8> = carry.drain(..take).collect();
        // Recorder stop chord: consume, never forward.
        if let Some(pos) = beat.iter().position(|&b| b == REC_FINISH_BYTE) {
            beat.truncate(pos);
            finished = true;
        }
        // Prior beat's inputs go out before this beat is measured.
        flush_typing(&mut pending, &mut type_run);
        for step in pending.drain(..) {
            steps.push(step);
        }
        let settled = stabilize(&mut registry, &id, interactive);
        emit_wait_for(&mut steps, &last_stable_lines(&last_stable), &settled);
        last_stable = settled;
        // Parse complete keys; the incomplete tail goes back to the front
        // of the buffer for the next beat. Original bytes are forwarded
        // verbatim so replay matches the session byte-for-byte.
        let mut offset = 0;
        while let Some((key, len)) = decode_key(&beat[offset..]) {
            let raw = beat[offset..offset + len].to_vec();
            offset += len;
            if let Ok(session) = registry.get_mut(&id) {
                let _ = session.pty.write_all(&raw);
            }
            match key {
                Key::Char(c) => type_run.push(c),
                Key::Named(name) => {
                    flush_typing(&mut pending, &mut type_run);
                    pending.push(Step::Press(name.to_string()));
                }
                Key::Ctrl(c) => {
                    flush_typing(&mut pending, &mut type_run);
                    pending.push(Step::Press(format!("CTRL+{c}")));
                }
                Key::Alt(c) => {
                    flush_typing(&mut pending, &mut type_run);
                    pending.push(Step::Press(format!("ALT+{c}")));
                }
            }
        }
        let mut buffered = beat[offset..].to_vec();
        buffered.extend(std::mem::take(&mut carry));
        carry = buffered;
        render(&mut registry, &id, interactive);
    }

    // EOF: let the app catch up with the final inputs (piped scripts land
    // long before boot), then flush, settle once more, write the suite.
    last_stable = drain_until_changed(
        &mut registry,
        &id,
        interactive,
        &last_stable,
        Duration::from_secs(5),
    );
    flush_typing(&mut pending, &mut type_run);
    for step in pending.drain(..) {
        steps.push(step);
    }
    coalesce_typing(&mut steps);
    if !carry.is_empty() {
        // Lone trailing ESC becomes an explicit press; anything else we
        // could never complete is counted, never silently kept.
        if carry == [0x1b] {
            steps.push(Step::Press("ESC".to_string()));
        } else {
            dropped_bytes += carry.len();
        }
        carry.clear();
    }
    let settled = stabilize(&mut registry, &id, interactive);
    emit_wait_for(&mut steps, &last_stable_lines(&last_stable), &settled);
    if dropped_bytes > 0 {
        eprintln!("warning: dropped {dropped_bytes} unparseable trailing bytes");
    }

    let name = out
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "recorded".to_string());
    let file = TestFile {
        schema: TestFile::schema_id().to_string(),
        name,
        application: tui_lab_protocol::Application {
            command: command.to_string(),
            args: args.to_vec(),
            cwd: None,
        },
        environment: Default::default(),
        terminal: tui_lab_protocol::TerminalConfig {
            width,
            height,
            timeout: None,
        },
        setup: Vec::new(),
        steps,
        cleanup: Vec::new(),
        assertions: vec![SuiteAssertion::ExitCode(0)],
    };
    match serde_yaml::to_string(&file) {
        Ok(yaml) => match std::fs::write(out, yaml) {
            Ok(()) => {
                println!("wrote {}", out.display());
                EXIT_OK
            }
            Err(e) => {
                eprintln!("record: write {}: {e}", out.display());
                EXIT_CONFIG_ERROR
            }
        },
        Err(e) => {
            eprintln!("record: encode suite: {e}");
            EXIT_CONFIG_ERROR
        }
    }
}

/// Move the typing buffer into a single `type` step.
fn flush_typing(pending: &mut Vec<Step>, type_run: &mut String) {
    if !type_run.is_empty() {
        pending.push(Step::Type(std::mem::take(type_run)));
    }
}

/// Merge adjacent `type` steps split by beat boundaries back into one.
/// Chunking is a pacing artifact, not user intent — the YAML should read as
/// the user typed.
fn coalesce_typing(steps: &mut Vec<Step>) {
    let mut merged: Vec<Step> = Vec::with_capacity(steps.len());
    for step in steps.drain(..) {
        match (merged.last_mut(), step) {
            (Some(Step::Type(into)), Step::Type(more)) => into.push_str(&more),
            (_, step) => merged.push(step),
        }
    }
    *steps = merged;
}

/// Pump until the screen hash holds still, then return the text.
fn stabilize(registry: &mut SessionRegistry, id: &str, interactive: bool) -> String {
    let start = Instant::now();
    let timeout = Duration::from_millis(REC_STABILIZE_TIMEOUT_MS);
    let mut last = String::new();
    let mut steady = 0;
    while start.elapsed() < timeout {
        let current = match registry.get_mut(id) {
            Ok(session) => SessionRegistry::pump_once(session, Duration::from_millis(REC_TICK_MS)),
            Err(_) => break,
        };
        render_text(&current, interactive);
        if current == last {
            steady += 1;
            if steady >= REC_STABLE_POLLS {
                break;
            }
        } else {
            steady = 0;
            last = current;
        }
    }
    last
}

/// Pump until the screen differs from `baseline` (or timeout): lets a slow
/// boot finish and lets piped input take effect before we measure. Returns
/// the latest text either way — callers stabilize after this.
fn drain_until_changed(
    registry: &mut SessionRegistry,
    id: &str,
    interactive: bool,
    baseline: &str,
    timeout: Duration,
) -> String {
    let start = Instant::now();
    let mut current = String::new();
    while start.elapsed() < timeout {
        current = match registry.get_mut(id) {
            Ok(session) => SessionRegistry::pump_once(session, Duration::from_millis(REC_TICK_MS)),
            Err(_) => break,
        };
        render_text(&current, interactive);
        if current.trim() != baseline.trim() && !current.trim().is_empty() {
            break;
        }
    }
    current
}

/// Emit a `wait_for_text` for the first new stable line, if any.
fn emit_wait_for(steps: &mut Vec<Step>, before_lines: &[String], after: &str) {
    if let Some(target) = first_new_line(before_lines, after) {
        steps.push(Step::WaitForText(WaitForText {
            text: target,
            regex: false,
            timeout: None,
        }));
    }
}

/// Trimmed non-empty lines of a screen dump, cleaned for wait targets.
fn last_stable_lines(text: &str) -> Vec<String> {
    text.lines().map(clean_line).collect()
}

/// Reduce a terminal row to its human content: drop padding, box-drawing
/// borders, and the list highlight marker. Turns `│> alpha-table   │` into
/// `alpha-table` — the string a human would assert on.
fn clean_line(line: &str) -> String {
    let trimmed = line.trim_matches(|c: char| {
        c.is_whitespace() || matches!(c, '│' | '┌' | '┐' | '└' | '┘' | '─')
    });
    trimmed
        .strip_prefix("> ")
        .unwrap_or(trimmed)
        .trim()
        .to_string()
}

/// First screen line (short, non-empty) absent from the previous screen.
fn first_new_line(before: &[String], after: &str) -> Option<String> {
    after
        .lines()
        .map(clean_line)
        .filter(|line| !line.is_empty() && line.len() <= 100)
        .find(|line| !before.iter().any(|old| old == line))
}

/// Pump once and repaint the preview (interactive terminals only).
fn render(registry: &mut SessionRegistry, id: &str, interactive: bool) {
    if let Ok(session) = registry.get_mut(id) {
        let text = SessionRegistry::pump_once(session, Duration::from_millis(REC_TICK_MS));
        render_text(&text, interactive);
    }
}

/// Repaint helper honoring pipe mode (no clear codes into logs).
fn render_text(text: &str, interactive: bool) {
    if !interactive {
        return;
    }
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(b"\x1b[2J\x1b[H");
    let _ = out.write_all(text.as_bytes());
    let _ = out.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleaning_yields_assertable_text() {
        assert_eq!(clean_line("┌TUI-LAB-SAMPLE──┐"), "TUI-LAB-SAMPLE");
        assert_eq!(clean_line("│> alpha-table   │"), "alpha-table");
        assert_eq!(clean_line("│Search: table   │"), "Search: table");
        assert_eq!(clean_line("   "), "");
    }

    #[test]
    fn new_lines_ignore_borders() {
        let before = last_stable_lines("┌TUI-LAB-SAMPLE──┐\n│> alpha-table │");
        let after = "┌TUI-LAB-SAMPLE──┐\n│  alpha-table │\n│> gamma-table │";
        assert_eq!(
            first_new_line(&before, after),
            Some("gamma-table".to_string())
        );
    }
}
