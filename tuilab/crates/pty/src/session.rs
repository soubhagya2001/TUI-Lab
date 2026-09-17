//! PTY session: spawn a child under Unix PTY / Windows ConPTY,
//! pump its output on a reader thread, resize, and close with a bound.
//!
//! Design notes (docs/03, docs/12 §12.4):
//! * A blocking `wait()` can hang the runner — [`PtySession::close`] always
//!   bounds the wait and kills on timeout.
//! * The child’s stdout arrives here as raw bytes; grid parsing lives in
//!   `tui-lab-terminal`. This crate never interprets escape sequences.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};

use crate::constants::{DEFAULT_HEIGHT, DEFAULT_WIDTH, KILL_GRACE_DEFAULT_MS};
use crate::error::{PtyError, Result};
use crate::utils::term_for;

/// Options for spawning a child under a fresh PTY.
pub struct SpawnOptions {
    /// Executable, e.g. `"./myapp"` or `"cmd"`.
    pub command: String,
    /// CLI arguments.
    pub args: Vec<String>,
    /// Working directory. `None` inherits the current one.
    pub cwd: Option<PathBuf>,
    /// Extra environment variables.
    pub env: Vec<(String, String)>,
    /// Terminal width in columns.
    pub cols: u16,
    /// Terminal height in rows.
    pub rows: u16,
}

impl Default for SpawnOptions {
    fn default() -> Self {
        Self {
            command: String::new(),
            args: Vec::new(),
            cwd: None,
            env: Vec::new(),
            cols: DEFAULT_WIDTH,
            rows: DEFAULT_HEIGHT,
        }
    }
}

/// A live child process attached to a PTY.
pub struct PtySession {
    child: Box<dyn Child>,
    master: Box<dyn MasterPty>,
    output_rx: mpsc::Receiver<Vec<u8>>,
    writer: SharedWriter,
}

/// Shareable handle to the PTY master writer.
///
/// Handing a clone to `tui-lab-terminal` lets the emulator forward
/// `Event::PtyWrite` handshake replies (docs/12 §12.4). Same shape as the
/// terminal crate's sink so the two plug together without adaptation.
pub type SharedWriter = Arc<Mutex<Box<dyn Write + Send>>>;

impl PtySession {
    /// Spawn `opts.command` under a new PTY and start the output pump.
    pub fn spawn(opts: &SpawnOptions) -> Result<Self> {
        if opts.command.is_empty() {
            return Err(PtyError::Spawn("empty command".to_string()));
        }
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: opts.rows,
                cols: opts.cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::Spawn(e.to_string()))?;

        let mut cmd = CommandBuilder::new(&opts.command);
        cmd.args(&opts.args);
        match &opts.cwd {
            Some(cwd) => {
                cmd.cwd(cwd);
            }
            // ConPTY children do NOT inherit the parent working directory
            // unless set explicitly (they land in the profile dir), so
            // default to it here. Relative suite paths depend on this.
            None => {
                if let Ok(dir) = std::env::current_dir() {
                    cmd.cwd(dir);
                }
            }
        }
        let mut saw_term = false;
        for (key, value) in &opts.env {
            if key == "TERM" {
                saw_term = true;
            }
            cmd.env(key, value);
        }
        if !saw_term {
            cmd.env("TERM", term_for(None));
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| PtyError::Spawn(e.to_string()))?;

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| PtyError::Spawn(e.to_string()))?;
        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        std::thread::spawn(move || {
            let mut chunk = [0u8; 4096];
            loop {
                match reader.read(&mut chunk) {
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
        let writer: SharedWriter = Arc::new(Mutex::new(
            pair.master
                .take_writer()
                .map_err(|e| PtyError::Spawn(e.to_string()))?,
        ));

        Ok(Self {
            child,
            master: pair.master,
            output_rx: rx,
            writer,
        })
    }

    /// Cloneable writer handle for wiring into `tui-lab-terminal`.
    pub fn writer_sink(&self) -> SharedWriter {
        Arc::clone(&self.writer)
    }

    /// Write raw bytes to the child’s stdin.
    pub fn write_all(&mut self, bytes: &[u8]) -> Result<()> {
        self.writer
            .lock()
            .map_err(|e| PtyError::Io(e.to_string()))?
            .write_all(bytes)
            .map_err(|e| PtyError::Io(e.to_string()))
    }

    /// Collect output chunks until `deadline` elapses. Never blocks longer.
    pub fn poll(&self, deadline: Duration) -> Vec<u8> {
        let start = Instant::now();
        let mut out = Vec::new();
        while start.elapsed() < deadline {
            match self.output_rx.recv_timeout(Duration::from_millis(50)) {
                Ok(chunk) => out.extend_from_slice(&chunk),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        out
    }

    /// Collect output until `needle` appears or `deadline` elapses.
    pub fn poll_until(&self, needle: &str, deadline: Duration) -> Option<Vec<u8>> {
        let start = Instant::now();
        let mut out = Vec::new();
        while start.elapsed() < deadline {
            match self.output_rx.recv_timeout(Duration::from_millis(50)) {
                Ok(chunk) => out.extend_from_slice(&chunk),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
            if String::from_utf8_lossy(&out).contains(needle) {
                return Some(out);
            }
        }
        None
    }

    /// Resize the PTY (and notify the child via `SIGWINCH` / ConPTY resize).
    pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::Io(e.to_string()))
    }

    /// Non-blocking exit check.
    pub fn try_wait(&mut self) -> Result<Option<portable_pty::ExitStatus>> {
        self.child
            .try_wait()
            .map_err(|e| PtyError::Io(e.to_string()))
    }

    /// Bounded close: optionally send quit bytes, wait up to `grace`
    /// (defaults to [`KILL_GRACE_DEFAULT_MS`]), then kill.
    pub fn close(
        &mut self,
        quit: Option<&[u8]>,
        grace: Option<Duration>,
    ) -> Result<portable_pty::ExitStatus> {
        if let Some(quit) = quit {
            let _ = self.write_all(quit);
        }
        let grace = grace.unwrap_or(Duration::from_millis(KILL_GRACE_DEFAULT_MS));
        let start = Instant::now();
        loop {
            match self.try_wait()? {
                Some(status) => return Ok(status),
                None if start.elapsed() < grace => {
                    std::thread::sleep(Duration::from_millis(50));
                }
                None => {
                    self.child.kill().map_err(|e| PtyError::Io(e.to_string()))?;
                    return self.child.wait().map_err(|e| PtyError::Io(e.to_string()));
                }
            }
        }
    }
}
