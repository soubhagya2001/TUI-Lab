//! Multi-session registry: the shared home for live PTYs (docs/08 §8.4).
//!
//! One entry per `session_id`: the PTY session, its grid emulator, input
//! history, and last-activity stamp. Single-session `run_file` stays untouched;
//! the MCP server (Phase 4) and future transports share this registry instead
//! of rebuilding session logic (thin-adapter rule).

use std::collections::HashMap;
use std::time::{Duration, Instant};

use tui_lab_pty::{PtySession, SpawnOptions};
use tui_lab_terminal::Emulator;

use crate::error::{CoreError, Result};

/// Default cap on concurrent sessions (docs/08 §8.4).
pub const DEFAULT_MAX_SESSIONS: usize = 8;

/// A live application under test.
pub struct LiveSession {
    /// PTY child + output pump.
    pub pty: PtySession,
    /// Grid emulator fed from the pump.
    pub emu: Emulator,
    /// Human input history (`press ENTER`, …).
    pub input_history: Vec<String>,
    /// Last client-initiated access (idle reaping).
    ///
    /// Stamped by spawn, [`SessionRegistry::get_mut`] (every write path goes
    /// through it), and resize. R10: reads never stamp — a poll loop must
    /// not keep an abandoned session alive forever.
    pub last_active: Instant,
}

/// How a reaped session ended.
pub struct ClosedSession {
    /// True when the child exited on its own with success.
    pub exited_cleanly: bool,
    /// Numeric exit code as reported (1 on signal deaths per PTY convention).
    pub exit_code: u32,
    /// Termination signal name, if reported.
    pub signal: Option<String>,
    /// Kill-path evidence: pump counters when the grace expired first.
    /// `None` on natural exits. Exists so the next Hangup-style mystery
    /// arrives with data (bytes flowing vs pump dead) instead of theories.
    pub note: Option<String>,
}

impl std::fmt::Debug for LiveSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LiveSession")
            .field("history", &self.input_history)
            .field("last_active", &self.last_active)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for ClosedSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClosedSession")
            .field("exited_cleanly", &self.exited_cleanly)
            .field("exit_code", &self.exit_code)
            .field("signal", &self.signal)
            .field("note", &self.note)
            .finish()
    }
}

/// Spawn options plus grid geometry for a new session.
pub struct NewSession {
    /// PTY spawn options.
    pub spawn: SpawnOptions,
}

impl NewSession {
    /// Build from spawn options.
    pub fn new(spawn: SpawnOptions) -> Self {
        Self { spawn }
    }
}

/// Registry of live sessions.
pub struct SessionRegistry {
    sessions: HashMap<String, LiveSession>,
    next_id: u64,
    max: usize,
}

impl SessionRegistry {
    /// Empty registry with the default cap.
    pub fn new() -> Self {
        Self::with_cap(DEFAULT_MAX_SESSIONS)
    }

    /// Empty registry with an explicit cap.
    pub fn with_cap(max: usize) -> Self {
        Self {
            sessions: HashMap::new(),
            next_id: 1,
            max,
        }
    }

    /// Live session count.
    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    /// Whether no sessions are live.
    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }

    /// Spawn a child and register it; returns the new `session_id`.
    pub fn spawn(&mut self, opts: NewSession) -> Result<String> {
        if self.sessions.len() >= self.max {
            return Err(CoreError::Message(format!(
                "session limit ({}) reached; close one first",
                self.max
            )));
        }
        let pty = PtySession::spawn(&opts.spawn).map_err(|e| CoreError::Launch(e.to_string()))?;
        let (cols, rows) = (opts.spawn.cols as usize, opts.spawn.rows as usize);
        let mut emu = Emulator::new(cols, rows, pty.writer_sink());
        // Warm the grid THROUGH the emulator: the first bytes often carry the
        // ConPTY handshake (ESC[6n), whose reply unblocks all further output.
        // Draining without feeding stalls the child forever (docs/12 §12.4).
        let warmup = pty.poll(Duration::from_millis(100));
        emu.feed(&warmup);
        let id = crate::utils::session_id(self.next_id);
        self.next_id += 1;
        self.sessions.insert(
            id.clone(),
            LiveSession {
                pty,
                emu,
                input_history: Vec::new(),
                last_active: Instant::now(),
            },
        );
        Ok(id)
    }

    /// Mutable client access with activity stamp; errors name the unknown id.
    ///
    /// Writes (press/type), resizes, and in-flight waits all come through
    /// here, so a session a client is actively driving never idles out.
    pub fn get_mut(&mut self, id: &str) -> Result<&mut LiveSession> {
        let session = self
            .sessions
            .get_mut(id)
            .ok_or_else(|| CoreError::Message(format!("unknown session: {id}")))?;
        session.last_active = Instant::now();
        Ok(session)
    }

    /// Bounded close + removal with an optional graceful quit first.
    ///
    /// When `quit` is given, its bytes go to the child and `close` polls for
    /// natural exit within the grace period before falling back to kill.
    /// This removes the press-quit/close race: quitting and reaping happen in
    /// one call instead of two racy round-trips. Unknown ids are an error.
    pub fn remove(&mut self, id: &str, quit: Option<&[u8]>) -> Result<ClosedSession> {
        let mut session = self
            .sessions
            .remove(id)
            .ok_or_else(|| CoreError::Message(format!("unknown session: {id}")))?;
        let status = session
            .pty
            .close(quit, None)
            .map_err(|e| CoreError::Pty(e.to_string()))?;
        let (bytes, errors) = session.pty.pump_stats();
        let exited_cleanly = status.success();
        let signal = status.signal().map(str::to_string);
        let note = if !exited_cleanly && signal.is_some() {
            Some(format!(
                "killed after grace expired; pump delivered {bytes} bytes with {errors} read errors"
            ))
        } else {
            None
        };
        Ok(ClosedSession {
            exited_cleanly,
            exit_code: status.exit_code(),
            signal,
            note,
        })
    }

    /// Resize a session's PTY + grid together, returning the actual dims.
    ///
    /// Shared choke point for the runner, proto, and MCP paths: zero dims
    /// are rejected, oversize dims clamp to materialization limits (R6).
    pub fn resize(&mut self, id: &str, width: u16, height: u16) -> Result<(u16, u16)> {
        if width == 0 || height == 0 {
            return Err(CoreError::Message(format!(
                "resize needs nonzero dimensions, got {width}x{height}"
            )));
        }
        let session = self
            .sessions
            .get_mut(id)
            .ok_or_else(|| CoreError::Message(format!("unknown session: {id}")))?;
        let (width, height) = tui_lab_terminal::utils::clamp_dims(width, height);
        session
            .pty
            .resize(width, height)
            .map_err(|e| CoreError::Pty(e.to_string()))?;
        session.emu.resize(width as usize, height as usize);
        session.last_active = Instant::now();
        Ok((width, height))
    }

    /// Close and drop sessions idle longer than `max_idle`. Returns the count.
    pub fn reap_idle(&mut self, max_idle: Duration) -> usize {
        let stale: Vec<String> = self
            .sessions
            .iter()
            .filter(|(_, session)| session.last_active.elapsed() > max_idle)
            .map(|(id, _)| id.clone())
            .collect();
        let mut reaped = 0;
        for id in stale {
            if let Some(mut session) = self.sessions.remove(&id) {
                // R7: a failed reap-close must not stop the sweep — trace it.
                if let Err(e) = session.pty.close(None, Some(Duration::from_secs(2))) {
                    tracing::warn!("reap close of {id} failed: {e}");
                }
                reaped += 1;
            }
        }
        reaped
    }

    /// Poll the pump once, feed the grid, return the fresh screen text.
    ///
    /// The single choke point for live reads: handshake replies always flow
    /// because every byte passes through the emulator immediately.
    ///
    /// R10: reads never refresh `last_active`. Polling is not usage — the
    /// idle reaper must still see a client that stopped driving the session,
    /// even while a read-side loop keeps draining the PTY.
    pub fn pump_once(session: &mut LiveSession, budget: Duration) -> String {
        let chunk = session.pty.poll(budget);
        session.emu.feed(&chunk);
        session.emu.text()
    }
}

impl Default for SessionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Resize one live PTY + grid pair, returning the actual dims.
///
/// Shared by the registry (`SessionRegistry::resize`), the YAML runner, and
/// the proto handler: zero dims are rejected, oversize dims clamp to
/// materialization limits (R6). PTY and grid always move together.
pub fn resize_live(
    pty: &mut tui_lab_pty::PtySession,
    emu: &mut tui_lab_terminal::Emulator,
    width: u16,
    height: u16,
) -> Result<(u16, u16)> {
    if width == 0 || height == 0 {
        return Err(CoreError::Message(format!(
            "resize needs nonzero dimensions, got {width}x{height}"
        )));
    }
    let (width, height) = tui_lab_terminal::utils::clamp_dims(width, height);
    pty.resize(width, height)
        .map_err(|e| CoreError::Pty(e.to_string()))?;
    emu.resize(width as usize, height as usize);
    Ok((width, height))
}
