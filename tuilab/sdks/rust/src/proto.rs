//! JSON-lines sidecar transport: one action per stdin line, one reply out.
//!
//! Speaks the `tuilab proto` contract pinned by
//! `tuilab/crates/cli/tests/proto_roundtrip.rs` — an engine change that
//! breaks SDKs fails there first.

use std::path::Path;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use crate::binary::find_binary;
use crate::error::TuiLabError;

/// Reply timeout per request.
const REPLY_TIMEOUT: Duration = Duration::from_secs(60);

/// One long-lived `tuilab proto` process.
pub struct Connection {
    child: Child,
    stdin: ChildStdin,
    lines: tokio::io::Lines<BufReader<ChildStdout>>,
}

impl Connection {
    /// Spawn the sidecar. `tuilab proto` emits nothing until the first
    /// request, so there is no banner to wait for.
    pub async fn spawn(binary: Option<&Path>) -> Result<Self, TuiLabError> {
        let mut child = Command::new(find_binary(binary)?)
            .arg("proto")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .map_err(|e| TuiLabError::new(format!("spawn tuilab proto: {e}")))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| TuiLabError::new("no stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| TuiLabError::new("no stdout"))?;
        Ok(Self {
            child,
            stdin,
            lines: BufReader::new(stdout).lines(),
        })
    }

    /// Send one action, return the reply object.
    pub async fn request(
        &mut self,
        action: serde_json::Value,
    ) -> Result<serde_json::Value, TuiLabError> {
        let mut line = serde_json::to_string(&action)
            .map_err(|e| TuiLabError::new(format!("encode action: {e}")))?;
        line.push('\n');
        self.stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|e| TuiLabError::new(format!("write action: {e}")))?;
        let reply = tokio::time::timeout(REPLY_TIMEOUT, self.lines.next_line())
            .await
            .map_err(|_| TuiLabError::new("proto reply timed out"))?
            .map_err(|e| TuiLabError::new(format!("read reply: {e}")))?
            .ok_or_else(|| TuiLabError::new("tuilab proto closed stdout"))?;
        serde_json::from_str(&reply)
            .map_err(|e| TuiLabError::new(format!("proto returned non-JSON: {e}")))
    }

    /// EOF the engine and reap the process.
    pub async fn close(mut self) -> Result<(), TuiLabError> {
        use tokio::io::AsyncWriteExt as _;
        let _ = self.stdin.shutdown().await;
        match tokio::time::timeout(Duration::from_secs(10), self.child.wait()).await {
            Ok(Ok(_)) => Ok(()),
            _ => {
                let _ = self.child.kill().await;
                Ok(())
            }
        }
    }
}

/// Launch options, mirroring the other SDKs.
#[derive(Debug, Default)]
pub struct LaunchOptions {
    /// CLI arguments.
    pub args: Vec<String>,
    /// Working directory.
    pub cwd: Option<String>,
    /// Extra environment.
    pub env: std::collections::HashMap<String, String>,
    /// Terminal width.
    pub width: u16,
    /// Terminal height.
    pub height: u16,
    /// Sidecar binary override.
    pub binary: Option<std::path::PathBuf>,
}

impl LaunchOptions {
    /// Default 120x40 terminal.
    pub fn new() -> Self {
        Self {
            width: 120,
            height: 40,
            ..Default::default()
        }
    }
}
