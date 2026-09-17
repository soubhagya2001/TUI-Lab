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
    /// Last successful access (idle reaping).
    pub last_active: Instant,
}

/// How a reaped session ended.
pub struct ClosedSession {
    /// True when the child exited on its own with success.
    pub exited_cleanly: bool,
    /// Termination signal name, if reported.
    pub signal: Option<String>,
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
            .field("signal", &self.signal)
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

    /// Mutable access with activity stamp; errors name the unknown id.
    pub fn get_mut(&mut self, id: &str) -> Result<&mut LiveSession> {
        let session = self
            .sessions
            .get_mut(id)
            .ok_or_else(|| CoreError::Message(format!("unknown session: {id}")))?;
        session.last_active = Instant::now();
        Ok(session)
    }

    /// Bounded close + removal. Unknown ids are an error, not a no-op.
    pub fn remove(&mut self, id: &str) -> Result<ClosedSession> {
        let mut session = self
            .sessions
            .remove(id)
            .ok_or_else(|| CoreError::Message(format!("unknown session: {id}")))?;
        let status = session
            .pty
            .close(None, None)
            .map_err(|e| CoreError::Pty(e.to_string()))?;
        Ok(ClosedSession {
            exited_cleanly: status.success(),
            signal: status.signal().map(str::to_string),
        })
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
                let _ = session.pty.close(None, Some(Duration::from_secs(2)));
                reaped += 1;
            }
        }
        reaped
    }

    /// Poll the pump once, feed the grid, return the fresh screen text.
    ///
    /// The single choke point for live reads: handshake replies always flow
    /// because every byte passes through the emulator immediately.
    pub fn pump_once(session: &mut LiveSession, budget: Duration) -> String {
        let chunk = session.pty.poll(budget);
        session.emu.feed(&chunk);
        session.last_active = Instant::now();
        session.emu.text()
    }
}

impl Default for SessionRegistry {
    fn default() -> Self {
        Self::new()
    }
}
