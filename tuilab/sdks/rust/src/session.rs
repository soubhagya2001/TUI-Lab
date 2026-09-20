//! Async TUI session over the sidecar (docs/09 §9.3 usage).

use std::path::Path;

use serde_json::{json, Value};

use crate::binary::find_binary;
use crate::error::TuiLabError;
use crate::proto::{Connection, LaunchOptions};

fn check(action: &str, reply: Value) -> Result<Value, TuiLabError> {
    if reply.get("ok") != Some(&Value::Bool(true)) {
        return Err(TuiLabError::with_detail(
            format!(
                "{action} failed: {}",
                reply.get("error").unwrap_or(&Value::Null)
            ),
            reply,
        ));
    }
    Ok(reply)
}

/// One application under test: launch → interact → assert → close.
pub struct TuiTest {
    conn: Connection,
    session_id: String,
}

impl TuiTest {
    /// Launch an app under a fresh PTY; returns a live session.
    pub async fn launch(
        command: impl Into<String>,
        options: LaunchOptions,
    ) -> Result<Self, TuiLabError> {
        let binary = options.binary.clone();
        let mut conn = Connection::spawn(binary.as_deref()).await?;
        let reply = conn
            .request(json!({
                "action": "launch",
                "command": command.into(),
                "args": options.args,
                "cwd": options.cwd,
                "terminal": {"width": options.width, "height": options.height},
                "env": options.env,
            }))
            .await;
        let reply = match reply {
            Ok(reply) => check("launch", reply)?,
            Err(e) => {
                conn.close().await.ok();
                return Err(e);
            }
        };
        let session_id = reply
            .get("session_id")
            .and_then(Value::as_str)
            .ok_or_else(|| TuiLabError::with_detail("launch: no session_id", reply.clone()))?
            .to_string();
        Ok(Self { conn, session_id })
    }

    /// Session id for log correlation.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    async fn act(&mut self, action: Value, name: &str) -> Result<Value, TuiLabError> {
        let reply = self.conn.request(action).await?;
        check(name, reply)
    }

    /// Send a named key; returns whether the screen changed.
    pub async fn press(&mut self, key: &str) -> Result<bool, TuiLabError> {
        let reply = self
            .act(
                json!({"action": "press", "session_id": self.session_id, "key": key}),
                "press",
            )
            .await?;
        Ok(reply.get("screen_changed") == Some(&Value::Bool(true)))
    }

    /// Type text verbatim (`sensitive` redacts it from logs).
    pub async fn type_text(&mut self, text: &str, sensitive: bool) -> Result<(), TuiLabError> {
        self.act(
            json!({"action": "type", "session_id": self.session_id, "text": text, "sensitive": sensitive}),
            "type",
        )
        .await?;
        Ok(())
    }

    /// Current screen grid (text, cursor, dimensions).
    pub async fn screen(&mut self) -> Result<Value, TuiLabError> {
        self.act(
            json!({"action": "screen", "session_id": self.session_id}),
            "screen",
        )
        .await
    }

    /// Poll until text is visible; raise with the last screen on timeout.
    pub async fn expect_text(
        &mut self,
        text: &str,
        timeout_ms: u64,
        regex: bool,
    ) -> Result<String, TuiLabError> {
        let reply = self
            .act(
                json!({"action": "wait_for_text", "session_id": self.session_id,
                       "text": text, "regex": regex, "timeout_ms": timeout_ms}),
                "wait_for_text",
            )
            .await?;
        if reply.get("found") != Some(&Value::Bool(true)) {
            return Err(TuiLabError::with_detail(
                format!("timed out waiting for {text:?}"),
                reply,
            ));
        }
        Ok(reply
            .get("screen")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string())
    }

    /// Assert text is absent from the current screen.
    pub async fn expect_not_text(&mut self, text: &str) -> Result<(), TuiLabError> {
        let screen = self.screen().await?;
        let body = screen
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if body.contains(text) {
            return Err(TuiLabError::with_detail(
                format!("unexpected visible text {text:?}"),
                screen,
            ));
        }
        Ok(())
    }

    /// Capture a named snapshot (golden written on first use).
    pub async fn snapshot(&mut self, name: &str) -> Result<Value, TuiLabError> {
        self.act(
            json!({"action": "snapshot", "session_id": self.session_id, "name": name}),
            "snapshot",
        )
        .await
    }

    /// Resize the terminal.
    pub async fn resize(&mut self, width: u16, height: u16) -> Result<(), TuiLabError> {
        self.act(
            json!({"action": "resize", "session_id": self.session_id, "width": width, "height": height}),
            "resize",
        )
        .await?;
        Ok(())
    }

    /// Close the session and reap the child (and the sidecar).
    pub async fn close(mut self) -> Result<Value, TuiLabError> {
        let reply = self
            .act(
                json!({"action": "close", "session_id": self.session_id}),
                "close",
            )
            .await;
        self.conn.close().await.ok();
        reply
    }
}

/// YAML suites through `tuilab run` (canonical format, docs/05).
pub struct Runner;

impl Runner {
    /// Run a suite file; return parsed results (raise on infra failures).
    pub async fn run(test_file: &Path, binary: Option<&Path>) -> Result<Vec<Value>, TuiLabError> {
        let output = tokio::process::Command::new(find_binary(binary)?)
            .arg("run")
            .arg(test_file)
            .output()
            .await
            .map_err(|e| TuiLabError::new(format!("spawn tuilab run: {e}")))?;
        let code = output.status.code().unwrap_or(1);
        if !matches!(code, 0 | 1) {
            return Err(TuiLabError::new(format!(
                "tuilab run exited {code}: {}",
                String::from_utf8_lossy(&output.stdout)
                    .chars()
                    .rev()
                    .take(2000)
                    .collect::<String>()
                    .chars()
                    .rev()
                    .collect::<String>()
            )));
        }
        // Results land next to the invoker's CWD (reports/results.json).
        let text = std::fs::read_to_string("reports/results.json")
            .map_err(|e| TuiLabError::new(format!("read results: {e}")))?;
        serde_json::from_str(&text).map_err(|e| TuiLabError::new(format!("parse results: {e}")))
    }
}
