//! `tuilab proto` wire contract: all 9 action shapes over JSON-lines.
//!
//! Pins the protocol every SDK programs against — an engine change that
//! breaks SDKs fails here first.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

/// Locate `target/debug/tuilab[.exe]` next to this test binary.
fn tuilab_bin() -> PathBuf {
    let mut path = std::env::current_exe().expect("test exe path");
    path.pop(); // deps
    path.pop(); // debug
    path.join(format!("tuilab{}", std::env::consts::EXE_SUFFIX))
}

/// Build the fixture binary on demand; return its path.
fn fixture_bin() -> String {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/ratatui-sample");
    let output = Command::new("cargo")
        .arg("build")
        .arg("--offline")
        .current_dir(&dir)
        .output()
        .expect("run cargo build for fixture");
    assert!(output.status.success(), "fixture build failed");
    let bin = if cfg!(windows) {
        "ratatui-sample.exe"
    } else {
        "ratatui-sample"
    };
    dir.join("target")
        .join("debug")
        .join(bin)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Pump thread: newline-delimited stdout into a channel.
fn pump(child: &mut std::process::Child) -> (mpsc::Sender<String>, mpsc::Receiver<String>) {
    let stdout = child.stdout.take().expect("stdout");
    let (out_tx, out_rx) = mpsc::channel::<String>();
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

fn ask(tx: &mpsc::Sender<String>, rx: &mpsc::Receiver<String>, line: String) -> serde_json::Value {
    tx.send(line).expect("send action");
    let text = rx
        .recv_timeout(Duration::from_secs(20))
        .expect("response in time");
    serde_json::from_str(&text).expect("response is JSON")
}

#[test]
fn proto_serves_all_nine_actions() {
    let root = std::env::temp_dir().join(format!("tuilab-proto-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("probe root");

    let mut child = Command::new(tuilab_bin())
        .arg("proto")
        .current_dir(&root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn tuilab proto");
    let (tx, rx) = pump(&mut child);

    // Unknown actions and garbage are JSON errors, never hangs.
    let bad = ask(&tx, &rx, r#"{"action":"dance"}"#.to_string());
    assert_eq!(bad["ok"], false);
    let bad = ask(&tx, &rx, "not json".to_string());
    assert_eq!(bad["ok"], false);

    // launch
    let launch = ask(
        &tx,
        &rx,
        format!(
            r#"{{"action":"launch","command":"{}","terminal":{{"width":120,"height":40}}}}"#,
            fixture_bin()
        ),
    );
    assert_eq!(launch["ok"], true, "{launch}");
    assert_eq!(launch["status"], "running");
    let sid = launch["session_id"]
        .as_str()
        .expect("session id")
        .to_string();
    assert!(sid.starts_with("sess_"));

    // wait_for_text
    let wait = ask(
        &tx,
        &rx,
        format!(
            r#"{{"action":"wait_for_text","session_id":"{sid}","text":"TUI-LAB-SAMPLE","timeout_ms":10000}}"#
        ),
    );
    assert_eq!(wait["found"], true, "{wait}");

    // press + type
    let press = ask(
        &tx,
        &rx,
        format!(r#"{{"action":"press","session_id":"{sid}","key":"DOWN"}}"#),
    );
    assert_eq!(press["ok"], true);
    let press = ask(
        &tx,
        &rx,
        format!(r#"{{"action":"press","session_id":"{sid}","key":"ENTER"}}"#),
    );
    assert_eq!(press["ok"], true);
    let typed = ask(
        &tx,
        &rx,
        format!(r#"{{"action":"type","session_id":"{sid}","text":"x","sensitive":true}}"#),
    );
    assert_eq!(typed["ok"], true);

    // screen
    let screen = ask(
        &tx,
        &rx,
        format!(r#"{{"action":"screen","session_id":"{sid}"}}"#),
    );
    assert_eq!(screen["ok"], true);
    assert!(
        screen["text"]
            .as_str()
            .unwrap_or_default()
            .contains("beta-chair"),
        "{screen}"
    );
    assert_eq!(screen["width"], 120);

    // assert
    let asserted = ask(
        &tx,
        &rx,
        format!(
            r#"{{"action":"assert","session_id":"{sid}","condition":{{"type":"text_visible","text":"beta-chair"}}}}"#
        ),
    );
    assert_eq!(asserted["passed"], true, "{asserted}");

    // snapshot writes the golden first, matches second.
    let snap = ask(
        &tx,
        &rx,
        format!(r#"{{"action":"snapshot","session_id":"{sid}","name":"proto-proof"}}"#),
    );
    assert_eq!(snap["saved"], true, "{snap}");
    let snap = ask(
        &tx,
        &rx,
        format!(r#"{{"action":"snapshot","session_id":"{sid}","name":"proto-proof"}}"#),
    );
    assert_eq!(snap["saved"], false, "{snap}");
    assert!(snap["diff"].is_null(), "{snap}");

    // resize then close a clean quit.
    let resize = ask(
        &tx,
        &rx,
        format!(r#"{{"action":"resize","session_id":"{sid}","width":80,"height":24}}"#),
    );
    assert_eq!(resize["ok"], true);
    let quit = ask(
        &tx,
        &rx,
        format!(r#"{{"action":"press","session_id":"{sid}","key":"q"}}"#),
    );
    assert_eq!(quit["ok"], true);
    let close = ask(
        &tx,
        &rx,
        format!(r#"{{"action":"close","session_id":"{sid}"}}"#),
    );
    assert_eq!(close["success"], true, "{close}");

    // Unknown sessions are errors.
    let gone = ask(
        &tx,
        &rx,
        r#"{"action":"screen","session_id":"sess_404"}"#.to_string(),
    );
    assert_eq!(gone["ok"], false);

    drop(tx);
    let _ = child.kill();
    let _ = std::fs::remove_dir_all(&root);
}
