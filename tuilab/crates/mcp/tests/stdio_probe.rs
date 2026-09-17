//! Live stdio probe: spawn `tuilab-mcp`, handshake, list tools.
//!
//! Proves the binary serves real MCP over stdio (initialize → tools/list).
//! Tool logic itself is covered in `mcp_tools.rs`; keep this about the wire.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

/// One pump thread turns the server's newline-delimited stdout into a channel.
fn pump(child: &mut std::process::Child) -> (mpsc::Sender<String>, mpsc::Receiver<String>) {
    let (out_tx, out_rx) = mpsc::channel::<String>();
    let stdout = child.stdout.take().expect("stdout");
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    if out_tx.send(line).is_err() {
                        break;
                    }
                }
            }
        }
    });
    let (in_tx, in_rx) = mpsc::channel::<String>();
    let mut stdin = child.stdin.take().expect("stdin");
    std::thread::spawn(move || {
        for line in in_rx {
            if stdin.write_all(format!("{line}\n").as_bytes()).is_err() {
                break;
            }
        }
    });
    (in_tx, out_rx)
}

fn recv(rx: &mpsc::Receiver<String>, what: &str) -> serde_json::Value {
    let text = rx
        .recv_timeout(Duration::from_secs(15))
        .unwrap_or_else(|_| panic!("no {what} from server"));
    serde_json::from_str(&text).unwrap_or_else(|_| panic!("{what} is not JSON: {text}"))
}
/// Locate `target/debug/tuilab-mcp[.exe]` next to this test binary.
fn server_bin() -> std::path::PathBuf {
    let mut path = std::env::current_exe().expect("test exe path");
    path.pop(); // deps
    path.pop(); // debug
    path.join(format!("tuilab-mcp{}", std::env::consts::EXE_SUFFIX))
}

#[test]
fn stdio_handshake_lists_nine_tools() {
    let root = std::env::temp_dir().join(format!("tuilab-probe-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("probe root");

    let mut child = Command::new(server_bin())
        .current_dir(&root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn tuilab-mcp");
    let (tx, rx) = pump(&mut child);

    tx.send(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"probe","version":"0.1.0"}}}"#.to_string())
        .expect("send initialize");
    let init = recv(&rx, "initialize response");
    assert_eq!(init["id"], 1);
    assert!(init.get("result").is_some(), "initialize answered: {init}");

    tx.send(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#.to_string())
        .expect("send initialized");
    tx.send(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#.to_string())
        .expect("send tools/list");
    let list = recv(&rx, "tools/list response");
    let tools = list["result"]["tools"].as_array().expect("tools array");
    assert_eq!(tools.len(), 9, "{list}");
    let mut names: Vec<&str> = tools
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    names.sort_unstable();
    assert!(names.contains(&"tui_launch"));
    assert!(names.contains(&"tui_run_test"));

    let _ = child.kill();
    let _ = std::fs::remove_dir_all(&root);
}
