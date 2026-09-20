//! `tuilab proto`: JSON-lines engine mode for SDK sidecars (docs/09).
//!
//! Stdin: one JSON action per line (the `docs/04` shapes, validated by the
//! shared `ACTION_FIELDS` table). Stdout: one JSON response per line.
//! Stderr: human logs. Tracing never touches stdout — the transport owns it.
//!
//! Response envelopes: success is `{"ok": true, ...fields}`; failures are
//! `{"ok": false, "error": "..."}`. One session registry per process.

use std::path::{Path, PathBuf};
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tui_lab_assertions::{condition_from_json, evaluate};
use tui_lab_core::{NewSession, SessionRegistry};
use tui_lab_input::{encode_key, encode_text};
use tui_lab_protocol::Action;
use tui_lab_pty::SpawnOptions;
use tui_lab_runtime::wait_for_text;

use crate::constants::{EXIT_OK, MAX_PROTO_LINE_BYTES};

/// Serve JSON-lines on stdio until EOF. Returns the CLI exit code.
pub async fn serve() -> i32 {
    let mut registry = SessionRegistry::new();
    let snapshot_base = PathBuf::from("tests/snapshots/proto");
    let stdin = tokio::io::stdin();
    let mut lines = BufReader::new(stdin).lines();
    let mut stdout = tokio::io::stdout();

    loop {
        let line = match lines.next_line().await {
            Ok(Some(line)) => line,
            Ok(None) => break, // EOF: SDK closed us down.
            Err(e) => {
                respond(&mut stdout, &error_response(&format!("stdin: {e}"))).await;
                return EXIT_OK;
            }
        };
        if line.len() > MAX_PROTO_LINE_BYTES {
            respond(&mut stdout, &error_response("line too long")).await;
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        let response = dispatch(&mut registry, &snapshot_base, &line).await;
        respond(&mut stdout, &response).await;
    }
    EXIT_OK
}

async fn respond(stdout: &mut tokio::io::Stdout, response: &str) {
    let _ = stdout.write_all(response.as_bytes()).await;
    let _ = stdout.write_all(b"\n").await;
    let _ = stdout.flush().await;
}

fn error_response(message: &str) -> String {
    serde_json::json!({"ok": false, "error": message}).to_string()
}

/// Execute one JSON action line against the registry.
async fn dispatch(registry: &mut SessionRegistry, snapshot_base: &Path, line: &str) -> String {
    let action = match Action::from_json(line) {
        Ok(action) => action,
        Err(e) => return error_response(&e.to_string()),
    };
    match action {
        Action::Launch {
            command,
            args,
            cwd,
            terminal,
            env,
            ..
        } => {
            let (cols, rows) = terminal
                .map(|size| (size.width, size.height))
                .unwrap_or((120, 40));
            let id = match registry.spawn(NewSession::new(SpawnOptions {
                command,
                args,
                cwd: cwd.map(PathBuf::from),
                env: env.into_iter().collect(),
                cols,
                rows,
            })) {
                Ok(id) => id,
                Err(e) => return error_response(&e.to_string()),
            };
            let screen = match registry.get_mut(&id) {
                Ok(session) => SessionRegistry::pump_once(session, Duration::from_millis(500)),
                Err(e) => return error_response(&e.to_string()),
            };
            serde_json::json!({
                "ok": true,
                "session_id": id,
                "status": "running",
                "screen": screen,
            })
            .to_string()
        }
        Action::Press { session_id, key } => {
            let session = match registry.get_mut(&session_id) {
                Ok(session) => session,
                Err(e) => return error_response(&e.to_string()),
            };
            let bytes = match encode_key(&key) {
                Ok(bytes) => bytes,
                Err(e) => return error_response(&format!("{key:?}: {e}")),
            };
            let before = session.emu.text();
            if let Err(e) = session.pty.write_all(&bytes) {
                return error_response(&e.to_string());
            }
            session.input_history.push(format!("press {key}"));
            let text = SessionRegistry::pump_once(session, Duration::from_millis(300));
            serde_json::json!({"ok": true, "screen_changed": text != before}).to_string()
        }
        Action::Type {
            session_id,
            text,
            sensitive,
        } => {
            let session = match registry.get_mut(&session_id) {
                Ok(session) => session,
                Err(e) => return error_response(&e.to_string()),
            };
            if let Err(e) = session.pty.write_all(&encode_text(&text)) {
                return error_response(&e.to_string());
            }
            // Never echo sensitive text anywhere (logs carry length only).
            session.input_history.push(if sensitive {
                format!("type[len={}, redacted]", text.len())
            } else {
                format!("type {text:?}")
            });
            serde_json::json!({"ok": true}).to_string()
        }
        Action::WaitForText {
            session_id,
            text,
            regex,
            timeout_ms,
            poll_ms,
        } => {
            let session = match registry.get_mut(&session_id) {
                Ok(session) => session,
                Err(e) => return error_response(&e.to_string()),
            };
            let outcome = wait_for_text(
                || SessionRegistry::pump_once(session, Duration::from_millis(50)),
                &text,
                regex,
                Duration::from_millis(timeout_ms),
                Duration::from_millis(poll_ms),
            )
            .await;
            serde_json::json!({
                "ok": true,
                "found": outcome.found,
                "elapsed_ms": outcome.elapsed.as_millis() as u64,
                "screen": outcome.last_screen,
            })
            .to_string()
        }
        Action::Screen {
            session_id,
            styled: _,
        } => {
            let session = match registry.get_mut(&session_id) {
                Ok(session) => session,
                Err(e) => return error_response(&e.to_string()),
            };
            let text = SessionRegistry::pump_once(session, Duration::from_millis(100));
            let (width, height) = session.emu.dims();
            let (row, col) = session.emu.cursor();
            serde_json::json!({
                "ok": true,
                "width": width,
                "height": height,
                "cursor": {"row": row, "col": col},
                "text": text,
            })
            .to_string()
        }
        Action::Assert {
            session_id,
            condition,
        } => {
            let session = match registry.get_mut(&session_id) {
                Ok(session) => session,
                Err(e) => return error_response(&e.to_string()),
            };
            let condition = match condition_from_json(&condition) {
                Ok(condition) => condition,
                Err(e) => return error_response(&e),
            };
            let text = SessionRegistry::pump_once(session, Duration::from_millis(100));
            let (exit_code, crashed) = match session.pty.try_wait() {
                Ok(Some(status)) if status.success() => (Some(0), false),
                Ok(Some(_)) => (None, true),
                _ => (None, false),
            };
            let view = tui_lab_assertions::ScreenView {
                text,
                cursor: session.emu.cursor(),
                screen_changed: true,
                exit_code,
                crashed,
            };
            let verdict = evaluate(&condition, &view);
            serde_json::json!({
                "ok": true,
                "passed": verdict.passed,
                "detail": verdict.detail,
            })
            .to_string()
        }
        Action::Snapshot { session_id, name } => {
            let session = match registry.get_mut(&session_id) {
                Ok(session) => session,
                Err(e) => return error_response(&e.to_string()),
            };
            let text = SessionRegistry::pump_once(session, Duration::from_millis(100));
            let (width, height) = session.emu.dims();
            let golden = tui_lab_snapshots::text_golden_path(
                &snapshot_base.join(&session_id),
                &name,
                width as u16,
                height as u16,
            );
            if !golden.exists() {
                if let Err(e) = tui_lab_snapshots::save_text(
                    &snapshot_base.join(&session_id),
                    &name,
                    width as u16,
                    height as u16,
                    &text,
                ) {
                    return error_response(&e.to_string());
                }
                return serde_json::json!({"ok": true, "saved": true, "diff": null}).to_string();
            }
            let expected = match tui_lab_snapshots::load_text(&golden) {
                Ok(expected) => expected,
                Err(e) => return error_response(&e.to_string()),
            };
            let outcome = tui_lab_snapshots::compare_text(&expected, &text, &[]);
            serde_json::json!({
                "ok": true,
                "saved": false,
                "diff": (!outcome.equal).then_some(outcome.diff),
            })
            .to_string()
        }
        Action::Resize {
            session_id,
            width,
            height,
        } => {
            let session = match registry.get_mut(&session_id) {
                Ok(session) => session,
                Err(e) => return error_response(&e.to_string()),
            };
            if let Err(e) = session.pty.resize(width, height) {
                return error_response(&e.to_string());
            }
            session.emu.resize(width as usize, height as usize);
            serde_json::json!({"ok": true}).to_string()
        }
        Action::Close {
            session_id, signal, ..
        } => match registry.remove(&session_id, signal.as_deref().map(str::as_bytes)) {
            Ok(closed) => serde_json::json!({
                "ok": true,
                "success": closed.exited_cleanly,
                "exit_code": closed.exit_code,
                "signal": closed.signal,
                "detail": closed.note,
            })
            .to_string(),
            Err(e) => error_response(&e.to_string()),
        },
    }
}
