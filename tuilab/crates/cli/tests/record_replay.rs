//! Recorder proof: scripted stdin in, YAML out, replay green.
//!
//! Pipes a fixed key script into `tuilab record`, then runs the emitted
//! suite with `tuilab run`. No sleeps anywhere — beats are paced by the
//! recorder's stabilize gate, waits by the engine.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

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
        .current_dir(&dir)
        .output()
        .expect("run cargo build for fixture");
    assert!(
        output.status.success(),
        "fixture build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
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

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tuilab-record-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// Key script: DOWN, ENTER, /, t a b l e, ENTER, q.
fn script() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"\x1b[B"); // DOWN
    bytes.extend_from_slice(b"\r"); // ENTER
    bytes.extend_from_slice(b"/"); // search
    bytes.extend_from_slice(b"table"); // query
    bytes.extend_from_slice(b"\r"); // ENTER
    bytes.extend_from_slice(b"q"); // quit
    bytes
}

#[test]
fn record_emits_replayable_yaml_without_sleeps() {
    let dir = scratch("replay");
    let out = dir.join("recorded.yaml");

    let mut child = Command::new(tuilab_bin())
        .arg("record")
        .arg("--command")
        .arg(fixture_bin())
        .arg("--out")
        .arg(&out)
        .current_dir(&dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn tuilab record");
    child
        .stdin
        .as_mut()
        .expect("record stdin")
        .write_all(&script())
        .expect("write script");
    drop(child.stdin.take());
    let status = child.wait().expect("record exits");
    assert!(status.success(), "record exits 0");
    assert!(out.is_file(), "YAML written");

    let yaml = std::fs::read_to_string(&out).expect("read YAML");
    // Smart waits, never sleeps — the whole point of the recorder.
    assert!(yaml.contains("wait_for_text"), "has synthesized waits");
    assert!(!yaml.contains("sleep:"), "emits no sleeps");
    assert!(yaml.contains("DOWN"), "records navigation");
    // Chunking may split one typing run across beats; concatenated type
    // steps must still carry the full query.
    let typed: String = yaml
        .lines()
        .filter_map(|line| line.trim().strip_prefix("- type: "))
        .collect();
    assert!(typed.contains("table"), "records typed query");

    // The emitted suite replays green through the real runner.
    let replay = Command::new(tuilab_bin())
        .arg("run")
        .arg(&out)
        .current_dir(&dir)
        .output()
        .expect("run recorded suite");
    assert!(
        replay.status.success(),
        "replay green:\n{}",
        String::from_utf8_lossy(&replay.stdout)
    );
    let _ = std::fs::remove_dir_all(&dir);
}
