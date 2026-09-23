//! Trace viewer format: `trace.zip` with timeline + raw bytes (P5-B1).
//!
//! A trace is built from a finished [`SuiteResult`](tui_lab_core::SuiteResult)
//! (in-memory `trace` bytes included) and holds:
//! * `trace.json` — schema id, suite, terminal, per-step timeline
//!   (`started_ms`/`duration_ms`), failure evidence, chunk index;
//! * `pty.bin` — concatenated raw PTY bytes backing paced replay.
//!
//! The zip uses stored (uncompressed) entries with zeroed timestamps, so
//! bytes are deterministic and golden-testable. No new dependencies: the
//! format needs only local headers + a central directory (CRC32 bitwise).

use tui_lab_core::SuiteResult;

/// Trace schema id.
pub const TRACE_SCHEMA: &str = "tui-lab/trace-v1";

/// One timeline row.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TraceStep {
    /// Zero-based step index.
    pub index: usize,
    /// Human step description.
    pub kind: String,
    /// Milliseconds from run start to step start.
    pub started_ms: u64,
    /// Step duration in milliseconds.
    pub duration_ms: u64,
    /// Whether the step passed.
    pub passed: bool,
    /// Verdict detail.
    pub detail: String,
}

/// Parsed trace (timeline + raw bytes for replay).
#[derive(Debug, Clone, PartialEq)]
pub struct Trace {
    /// Suite name.
    pub suite: String,
    /// Timeline rows.
    pub steps: Vec<TraceStep>,
    /// Terminal geometry + type.
    pub terminal: (u16, u16, String),
    /// Failure evidence, when the run failed.
    pub failure: Option<TraceFailure>,
    /// True when byte capture hit the cap (replay is a prefix).
    pub truncated: bool,
    /// Raw PTY bytes backing replay.
    pub pty_bytes: Vec<u8>,
    /// Millisecond offsets parallel to `pty_bytes` chunk boundaries.
    pub chunk_at_ms: Vec<u64>,
    /// Byte lengths parallel to `chunk_at_ms`.
    pub chunk_lens: Vec<usize>,
    /// Input beats with timestamps (P5-E2 paced input replay).
    pub inputs: Vec<TraceInput>,
}

/// One recorded input write inside a trace.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TraceInput {
    /// Milliseconds from run start.
    pub at_ms: u64,
    /// Raw bytes written to the PTY.
    pub bytes: Vec<u8>,
}

/// Failure evidence inside a trace.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TraceFailure {
    /// Step index that failed.
    pub step_index: usize,
    /// Human step description.
    pub step: String,
    /// What was expected.
    pub expected: String,
    /// Last full screen.
    pub last_screen: String,
}

/// Build the `trace.json` document for a finished suite result.
pub fn trace_document(result: &SuiteResult) -> serde_json::Value {
    let steps: Vec<serde_json::Value> = result
        .steps
        .iter()
        .map(|step| {
            serde_json::json!({
                "index": step.index,
                "kind": step.kind,
                "started_ms": step.started_ms,
                "duration_ms": step.duration_ms,
                "passed": step.passed,
                "detail": step.detail,
            })
        })
        .collect();
    serde_json::json!({
        "schema": TRACE_SCHEMA,
        "suite": result.suite,
        "terminal": {
            "width": result.terminal.width,
            "height": result.terminal.height,
            "term": result.terminal.term,
        },
        "truncated": result.trace_truncated,
        "steps": steps,
        "inputs": result.input_trace.iter().map(|beat| {
            serde_json::json!({"at_ms": beat.at_ms, "len": beat.bytes.len()})
        }).collect::<Vec<_>>(),
        "failure": result.failure.as_ref().map(|failure| {
            serde_json::json!({
                "step_index": failure.step_index,
                "step": failure.step,
                "expected": failure.expected,
                "last_screen": failure.last_screen,
            })
        }),
        "chunks": result.trace.iter().map(|chunk| {
            serde_json::json!({"at_ms": chunk.at_ms, "len": chunk.bytes.len()})
        }).collect::<Vec<_>>(),
    })
}

/// Write `trace.zip` for a finished suite result.
pub fn write_trace(path: &std::path::Path, result: &SuiteResult) -> std::io::Result<()> {
    let document = trace_document(result);
    let json = serde_json::to_vec_pretty(&document)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let mut pty_bytes = Vec::new();
    for chunk in &result.trace {
        pty_bytes.extend_from_slice(&chunk.bytes);
    }
    let mut input_bytes = Vec::new();
    for beat in &result.input_trace {
        input_bytes.extend_from_slice(&beat.bytes);
    }
    let files = [
        ("trace.json", json.as_slice()),
        ("pty.bin", pty_bytes.as_slice()),
        ("inputs.bin", input_bytes.as_slice()),
    ];
    let bytes = zip_store(&files);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, bytes)
}

/// Read a `trace.zip` back into a [`Trace`].
pub fn read_trace(path: &std::path::Path) -> std::io::Result<Trace> {
    let bytes = std::fs::read(path)?;
    let files = zip_read(&bytes)?;
    let document: serde_json::Value = serde_json::from_slice(
        files
            .iter()
            .find(|(name, _)| *name == "trace.json")
            .map(|(_, data)| *data)
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "trace.json missing")
            })?,
    )
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    if document.get("schema").and_then(|s| s.as_str()) != Some(TRACE_SCHEMA) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unknown trace schema",
        ));
    }
    let steps = document
        .get("steps")
        .and_then(|steps| steps.as_array())
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "steps missing"))?
        .iter()
        .map(|step| TraceStep {
            index: step.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
            kind: step
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            started_ms: step.get("started_ms").and_then(|v| v.as_u64()).unwrap_or(0),
            duration_ms: step
                .get("duration_ms")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            passed: step
                .get("passed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            detail: step
                .get("detail")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        })
        .collect();
    let terminal = document.get("terminal");
    let failure = document.get("failure").and_then(|failure| {
        if failure.is_null() {
            return None;
        }
        Some(TraceFailure {
            step_index: failure
                .get("step_index")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize,
            step: failure
                .get("step")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            expected: failure
                .get("expected")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            last_screen: failure
                .get("last_screen")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        })
    });
    let mut chunk_at_ms = Vec::new();
    let mut chunk_lens = Vec::new();
    if let Some(chunks) = document.get("chunks").and_then(|c| c.as_array()) {
        for chunk in chunks {
            chunk_at_ms.push(chunk.get("at_ms").and_then(|v| v.as_u64()).unwrap_or(0));
            chunk_lens.push(chunk.get("len").and_then(|v| v.as_u64()).unwrap_or(0) as usize);
        }
    }
    let pty_bytes = files
        .iter()
        .find(|(name, _)| *name == "pty.bin")
        .map(|(_, data)| data.to_vec())
        .unwrap_or_default();
    let input_bytes = files
        .iter()
        .find(|(name, _)| *name == "inputs.bin")
        .map(|(_, data)| data.to_vec())
        .unwrap_or_default();
    let mut inputs = Vec::new();
    let mut input_offset = 0usize;
    if let Some(list) = document.get("inputs").and_then(|i| i.as_array()) {
        for beat in list {
            let at_ms = beat.get("at_ms").and_then(|v| v.as_u64()).unwrap_or(0);
            let len = beat.get("len").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let end = (input_offset + len).min(input_bytes.len());
            inputs.push(TraceInput {
                at_ms,
                bytes: input_bytes[input_offset..end].to_vec(),
            });
            input_offset = end;
        }
    }
    Ok(Trace {
        suite: document
            .get("suite")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        steps,
        terminal: (
            terminal
                .and_then(|t| t.get("width"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u16,
            terminal
                .and_then(|t| t.get("height"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u16,
            terminal
                .and_then(|t| t.get("term"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        ),
        failure,
        truncated: document
            .get("truncated")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        pty_bytes,
        chunk_at_ms,
        chunk_lens,
        inputs,
    })
}

/// Render a trace timeline as text (`tuilab trace`).
pub fn render_timeline(trace: &Trace) -> String {
    let mut out = format!("trace: {} ({} steps)\n", trace.suite, trace.steps.len());
    for step in &trace.steps {
        let mark = if step.passed { "✓" } else { "✗" };
        out.push_str(&format!(
            "  [{:>3}] +{:>6}ms {:>6}ms {mark} {}\n",
            step.index, step.started_ms, step.duration_ms, step.kind
        ));
    }
    if trace.truncated {
        out.push_str("  (byte capture hit the cap; replay is a prefix)\n");
    }
    if let Some(failure) = &trace.failure {
        out.push_str(&format!(
            "  failure at step {} ({}): expected {}\n",
            failure.step_index, failure.step, failure.expected
        ));
    }
    out
}

/// Replay schedule: `(bytes, sleep-before-ms)` pairs with gaps capped so
/// replay stays watchable. Pure and unit-tested; the CLI sleeps + writes.
pub fn replay_schedule(trace: &Trace, max_gap_ms: u64) -> Vec<(Vec<u8>, u64)> {
    let mut out = Vec::new();
    let mut offset = 0usize;
    let mut prev_at = 0u64;
    for (at_ms, len) in trace.chunk_at_ms.iter().zip(trace.chunk_lens.iter()) {
        let end = (offset + len).min(trace.pty_bytes.len());
        out.push((
            trace.pty_bytes[offset..end].to_vec(),
            at_ms.saturating_sub(prev_at).min(max_gap_ms),
        ));
        offset = end;
        prev_at = *at_ms;
    }
    out
}

/// Input replay schedule (P5-E2): `(bytes, sleep-before-ms)` pairs from the
/// recorded input beats, gaps capped at `max_gap_ms`. Pure and unit-tested;
/// the CLI/SDK sleeps + writes to reproduce paced typing.
pub fn input_replay_schedule(trace: &Trace, max_gap_ms: u64) -> Vec<(Vec<u8>, u64)> {
    let mut out = Vec::new();
    let mut prev_at = 0u64;
    for beat in &trace.inputs {
        out.push((
            beat.bytes.clone(),
            beat.at_ms.saturating_sub(prev_at).min(max_gap_ms),
        ));
        prev_at = beat.at_ms;
    }
    out
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// Minimal stored-entry zip (deterministic: zeroed timestamps).
fn zip_store(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in files {
        let header_offset = out.len() as u32;
        let crc = crc32(data);
        let size = data.len() as u32;
        // Local file header.
        push_u32(&mut out, 0x0403_4B50);
        push_u16(&mut out, 20);
        push_u16(&mut out, 0x0800); // UTF-8 names.
        push_u16(&mut out, 0); // Stored.
        push_u16(&mut out, 0); // mod time.
        push_u16(&mut out, 0x21); // mod date (1980-01-01).
        push_u32(&mut out, crc);
        push_u32(&mut out, size);
        push_u32(&mut out, size);
        push_u16(&mut out, name.len() as u16);
        push_u16(&mut out, 0);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);
        // Central directory entry.
        push_u32(&mut central, 0x0201_4B50);
        push_u16(&mut central, 20);
        push_u16(&mut central, 20);
        push_u16(&mut central, 0x0800);
        push_u16(&mut central, 0);
        push_u16(&mut central, 0);
        push_u16(&mut central, 0x21);
        push_u32(&mut central, crc);
        push_u32(&mut central, size);
        push_u32(&mut central, size);
        push_u16(&mut central, name.len() as u16);
        push_u16(&mut central, 0);
        push_u16(&mut central, 0);
        push_u16(&mut central, 0);
        push_u16(&mut central, 0);
        push_u32(&mut central, 0);
        push_u32(&mut central, header_offset);
        central.extend_from_slice(name.as_bytes());
    }
    let central_offset = out.len() as u32;
    let central_size = central.len() as u32;
    out.extend_from_slice(&central);
    // End of central directory.
    push_u32(&mut out, 0x0605_4B50);
    push_u16(&mut out, 0);
    push_u16(&mut out, 0);
    push_u16(&mut out, files.len() as u16);
    push_u16(&mut out, files.len() as u16);
    push_u32(&mut out, central_size);
    push_u32(&mut out, central_offset);
    push_u16(&mut out, 0);
    out
}

fn read_u16(data: &[u8], at: usize) -> std::io::Result<u16> {
    data.get(at..at + 2)
        .and_then(|b| b.try_into().ok())
        .map(u16::from_le_bytes)
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "short zip"))
}

fn read_u32(data: &[u8], at: usize) -> std::io::Result<u32> {
    data.get(at..at + 4)
        .and_then(|b| b.try_into().ok())
        .map(u32::from_le_bytes)
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "short zip"))
}

/// Parse stored entries: `[(name, data)]`. Rejects compressed entries.
fn zip_read(data: &[u8]) -> std::io::Result<Vec<(String, &[u8])>> {
    const EOCD: u32 = 0x0605_4B50;
    const CENTRAL: u32 = 0x0201_4B50;
    const LOCAL: u32 = 0x0403_4B50;
    if data.len() < 22 || read_u32(data, data.len() - 22)? != EOCD {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "missing end-of-central-directory",
        ));
    }
    let count = read_u16(data, data.len() - 14)? as usize;
    let central_offset = read_u32(data, data.len() - 6)? as usize;
    let mut out = Vec::with_capacity(count);
    let mut at = central_offset;
    for _ in 0..count {
        if read_u32(data, at)? != CENTRAL {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "bad central entry",
            ));
        }
        let method = read_u16(data, at + 10)?;
        if method != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "only stored entries supported",
            ));
        }
        let size = read_u32(data, at + 24)? as usize;
        let name_len = read_u16(data, at + 28)? as usize;
        let extra_len = read_u16(data, at + 30)? as usize;
        let comment_len = read_u16(data, at + 32)? as usize;
        let header_offset = read_u32(data, at + 42)? as usize;
        let name_start = at + 46;
        let name_end = name_start + name_len;
        let name = std::str::from_utf8(
            data.get(name_start..name_end)
                .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "bad name"))?,
        )
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "bad name"))?
        .to_string();
        // Local header mirrors name/extra lengths.
        let local_name_len = read_u16(data, header_offset + 26)? as usize;
        let local_extra_len = read_u16(data, header_offset + 28)? as usize;
        if read_u32(data, header_offset)? != LOCAL {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "bad local header",
            ));
        }
        let data_start = header_offset + 30 + local_name_len + local_extra_len;
        let data_end = data_start + size;
        let bytes = data
            .get(data_start..data_end)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "short data"))?;
        out.push((name, bytes));
        at = name_end + extra_len + comment_len;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_known_answers() {
        // Standard check values: a wrong polynomial breaks real unzip tools
        // even though our own reader ignores CRCs.
        assert_eq!(crc32(b"hello"), 0x3610_A686);
        assert_eq!(crc32(b""), 0x0000_0000);
    }

    #[test]
    fn zip_round_trips_stored_entries() {
        let files = [
            ("a.txt", b"hello".as_slice()),
            ("b.bin", b"\x00\xff".as_slice()),
        ];
        let bytes = zip_store(&files);
        // Deterministic: same input, same bytes (fixed timestamps).
        assert_eq!(bytes, zip_store(&files));
        let back = zip_read(&bytes).expect("parse");
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].0, "a.txt");
        assert_eq!(back[0].1, b"hello");
        assert_eq!(back[1].1, b"\x00\xff");
    }

    #[test]
    fn replay_schedule_caps_gaps() {
        let trace = Trace {
            suite: "x".to_string(),
            steps: Vec::new(),
            terminal: (80, 24, "xterm".to_string()),
            failure: None,
            truncated: false,
            pty_bytes: b"abcdef".to_vec(),
            chunk_at_ms: vec![0, 5000],
            chunk_lens: vec![2, 4],
            inputs: Vec::new(),
        };
        let schedule = replay_schedule(&trace, 1000);
        assert_eq!(schedule.len(), 2);
        assert_eq!(schedule[0], (b"ab".to_vec(), 0));
        assert_eq!(schedule[1], (b"cdef".to_vec(), 1000));
    }

    #[test]
    fn input_replay_schedule_caps_gaps() {
        let trace = Trace {
            suite: "x".to_string(),
            steps: Vec::new(),
            terminal: (80, 24, "xterm".to_string()),
            failure: None,
            truncated: false,
            pty_bytes: Vec::new(),
            chunk_at_ms: Vec::new(),
            chunk_lens: Vec::new(),
            inputs: vec![
                TraceInput {
                    at_ms: 0,
                    bytes: b"h".to_vec(),
                },
                TraceInput {
                    at_ms: 5000,
                    bytes: b"i".to_vec(),
                },
            ],
        };
        let schedule = input_replay_schedule(&trace, 1000);
        assert_eq!(schedule.len(), 2);
        assert_eq!(schedule[0], (b"h".to_vec(), 0));
        assert_eq!(schedule[1], (b"i".to_vec(), 1000));
    }
}
