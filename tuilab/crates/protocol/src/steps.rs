//! Portable YAML test definition, `schema: tui-lab/v1` (docs/05).
//!
//! Defined once here; the CLI (Phase 3) and MCP `tui_run_test` (Phase 4)
//! both execute these types. No frontend parses YAML itself.

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Deserializer, Serialize};

use crate::constants::SCHEMA_ID;
use crate::error::{ProtocolError, Result};
use crate::utils::is_supported_schema;

/// Top-level test file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestFile {
    /// Must be `tui-lab/v1`.
    pub schema: String,
    /// Suite name.
    pub name: String,
    /// Application under test.
    pub application: Application,
    /// Extra environment (`TERM`, …).
    #[serde(default)]
    pub environment: HashMap<String, String>,
    /// Terminal geometry + suite timeout.
    #[serde(default)]
    pub terminal: TerminalConfig,
    /// Steps run before the main flow.
    #[serde(default)]
    pub setup: Vec<Step>,
    /// Main flow.
    pub steps: Vec<Step>,
    /// Always run, even on failure.
    #[serde(default)]
    pub cleanup: Vec<Step>,
    /// End-of-suite assertions (e.g. `exit_code`).
    #[serde(default)]
    pub assertions: Vec<SuiteAssertion>,
}

impl TestFile {
    /// Parse + validate a YAML test file.
    pub fn from_yaml(text: &str) -> Result<Self> {
        let file: Self =
            serde_yaml::from_str(text).map_err(|e| ProtocolError::Schema(e.to_string()))?;
        if !is_supported_schema(&file.schema) {
            return Err(ProtocolError::Version(file.schema));
        }
        Ok(file)
    }

    /// Schema id guard shared with the JSON protocol.
    pub fn schema_id() -> &'static str {
        SCHEMA_ID
    }
}

/// Application under test (usually implicit `launch`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Application {
    /// Executable.
    pub command: String,
    /// CLI arguments.
    #[serde(default)]
    pub args: Vec<String>,
    /// Working directory.
    #[serde(default)]
    pub cwd: Option<String>,
}

/// Terminal geometry + suite timeout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalConfig {
    /// Columns.
    #[serde(default = "default_width")]
    pub width: u16,
    /// Rows.
    #[serde(default = "default_height")]
    pub height: u16,
    /// Suite timeout, e.g. `10s`.
    #[serde(default, with = "humantime_opt")]
    pub timeout: Option<Duration>,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            width: default_width(),
            height: default_height(),
            timeout: None,
        }
    }
}

fn default_width() -> u16 {
    120
}

fn default_height() -> u16 {
    40
}

/// One step in `setup` / `steps` / `cleanup` (docs/05 §5.3).
///
/// YAML form is a single-key map (`- press: ENTER`). Deserialization is manual
/// so aliases (`expect`/`screenshot`, `assert_exit_code`) and error messages
/// stay under our control; serialization is manual too, because serde's
/// default `!Variant` tags for newtype variants would not round-trip.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    /// Send a named key.
    Press(String),
    /// Type text verbatim.
    Type(String),
    /// Poll until text appears. Optional `timeout` (`2s` default policy).
    WaitForText(WaitForText),
    /// Escape hatch only — the recorder never emits this.
    Sleep(SleepFor),
    /// Resize the terminal.
    Resize(ResizeTo),
    /// Assert visible text.
    AssertText(TextAssertion),
    /// Legacy alias of `AssertText`.
    Expect(TextAssertion),
    /// Assert text inside a region.
    AssertRegion(RegionAssertion),
    /// Capture a named snapshot.
    Snapshot(SnapshotTake),
    /// Legacy alias of `Snapshot`.
    Screenshot(SnapshotTake),
    /// Wait for the process to exit.
    WaitForExit(WaitForExit),
}

/// `wait_for_text` payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaitForText {
    /// Substring (or regex with `regex: true`) to wait for.
    pub text: String,
    /// Regex mode.
    #[serde(default)]
    pub regex: bool,
    /// Timeout as a human string (`500ms`, `3s`).
    #[serde(default, with = "humantime_opt")]
    pub timeout: Option<Duration>,
}

/// `sleep` payload (discouraged escape hatch).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SleepFor(#[serde(with = "humantime")] pub Duration);

/// `resize` payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResizeTo {
    /// Columns.
    pub width: u16,
    /// Rows.
    pub height: u16,
}

/// `assert_text` / `expect` payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextAssertion {
    /// Must be visible.
    pub contains: Option<String>,
    /// Must be absent.
    pub not_contains: Option<String>,
    /// Regex that must match.
    pub regex: Option<String>,
}

/// `assert_region` payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegionAssertion {
    /// Left column.
    pub x: u16,
    /// Top row.
    pub y: u16,
    /// Region width.
    pub width: u16,
    /// Region height.
    pub height: u16,
    /// Text the region must contain.
    pub contains: String,
}

/// `snapshot` payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotTake {
    /// Snapshot name.
    pub name: String,
    /// Mask list (regex strings or region names).
    #[serde(default)]
    pub mask: Vec<String>,
}

/// `wait_for_exit` payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaitForExit {
    /// Timeout as a human string.
    #[serde(with = "humantime")]
    pub timeout: Duration,
}

/// End-of-suite assertions.
///
/// Accepts both `exit_code` and the `assert_*` aliases used across docs/05–06.
/// Serializes back to the same single-key-map shape it parses (serde's
/// default `!tag` form for scalar newtypes would not round-trip).
#[derive(Debug, Clone, PartialEq)]
pub enum SuiteAssertion {
    /// Expected process exit code.
    ExitCode(i32),
    /// Assert the process did not crash (signal death / nonzero via signal).
    ProcessNotCrashed(bool),
}

/// Minimal human-duration serde (`500ms`, `3s`, `2m`) without a new dependency.
mod humantime {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(value: &Duration, ser: S) -> Result<S::Ok, S::Error> {
        format!("{}ms", value.as_millis()).serialize(ser)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<Duration, D::Error> {
        let raw = String::deserialize(de)?;
        super::parse_duration(&raw).map_err(serde::de::Error::custom)
    }
}

/// Optional variant of [`humantime`].
mod humantime_opt {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(value: &Option<Duration>, ser: S) -> Result<S::Ok, S::Error> {
        match value {
            Some(duration) => super::humantime::serialize(duration, ser),
            None => ser.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<Option<Duration>, D::Error> {
        let raw: Option<String> = Option::deserialize(de)?;
        raw.map(|text| super::parse_duration(&text).map_err(serde::de::Error::custom))
            .transpose()
    }
}

/// Serialize back to the single-key-map shape this type parses.
impl Serialize for SuiteAssertion {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap as _;
        let mut map = ser.serialize_map(Some(1))?;
        match self {
            Self::ExitCode(code) => map.serialize_entry("exit_code", code)?,
            Self::ProcessNotCrashed(flag) => {
                map.serialize_entry("process_not_crashed", flag)?;
            }
        }
        map.end()
    }
}

/// Parse `500ms` / `3s` / `2m`.
fn parse_duration(raw: &str) -> std::result::Result<Duration, String> {
    let (number, unit) = raw
        .find(|c: char| c.is_alphabetic())
        .map_or((raw, ""), |idx| raw.split_at(idx));
    let value: u64 = number
        .trim()
        .parse()
        .map_err(|_| format!("bad duration: {raw}"))?;
    match unit.trim() {
        "ms" | "" => Ok(Duration::from_millis(value)),
        "s" => Ok(Duration::from_secs(value)),
        "m" => Ok(Duration::from_secs(value * 60)),
        other => Err(format!("bad duration unit in {raw}: {other}")),
    }
}

/// Serialize back to the single-key-map shape this type parses.
impl Serialize for Step {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap as _;
        let mut map = ser.serialize_map(Some(1))?;
        match self {
            Self::Press(key) => map.serialize_entry("press", key)?,
            Self::Type(text) => map.serialize_entry("type", text)?,
            Self::WaitForText(wait) => map.serialize_entry("wait_for_text", wait)?,
            Self::Sleep(sleep) => {
                map.serialize_entry("sleep", &format!("{}ms", sleep.0.as_millis()))?
            }
            Self::Resize(to) => map.serialize_entry("resize", to)?,
            Self::AssertText(assertion) | Self::Expect(assertion) => {
                map.serialize_entry("assert_text", assertion)?;
            }
            Self::AssertRegion(region) => map.serialize_entry("assert_region", region)?,
            Self::Snapshot(take) | Self::Screenshot(take) => {
                map.serialize_entry("snapshot", take)?;
            }
            Self::WaitForExit(wait) => map.serialize_entry("wait_for_exit", wait)?,
        }
        map.end()
    }
}

/// Deserialize one enum from a single-key map (`- press: ENTER`).
impl<'de> Deserialize<'de> for Step {
    fn deserialize<D: Deserializer<'de>>(de: D) -> std::result::Result<Self, D::Error> {
        let map = HashMap::<String, serde_yaml::Value>::deserialize(de)
            .map_err(|e| serde::de::Error::custom(format!("step must be a single-key map: {e}")))?;
        if map.len() != 1 {
            return Err(serde::de::Error::custom(format!(
                "step must have exactly one key, got {}",
                map.len()
            )));
        }
        let (key, value) = map.into_iter().next().expect("single step");
        match key.as_str() {
            "press" => convert::<String>(value).map(Step::Press),
            "type" => convert::<String>(value).map(Step::Type),
            "wait_for_text" => convert::<WaitForText>(value).map(Step::WaitForText),
            "sleep" => convert::<SleepFor>(value).map(Step::Sleep),
            "resize" => convert::<ResizeTo>(value).map(Step::Resize),
            "assert_text" => convert::<TextAssertion>(value).map(Step::AssertText),
            "expect" => convert::<TextAssertion>(value).map(Step::Expect),
            "assert_region" => convert::<RegionAssertion>(value).map(Step::AssertRegion),
            "snapshot" => convert::<SnapshotTake>(value).map(Step::Snapshot),
            "screenshot" => convert::<SnapshotTake>(value).map(Step::Screenshot),
            "wait_for_exit" => convert::<WaitForExit>(value).map(Step::WaitForExit),
            other => Err(format!("unknown step: {other}")),
        }
        .map_err(serde::de::Error::custom)
    }
}

/// Convert one YAML value into its payload type.
fn convert<T: for<'de> Deserialize<'de>>(
    value: serde_yaml::Value,
) -> std::result::Result<T, String> {
    serde_yaml::from_value(value).map_err(|e| e.to_string())
}

impl<'de> Deserialize<'de> for SuiteAssertion {
    fn deserialize<D: Deserializer<'de>>(de: D) -> std::result::Result<Self, D::Error> {
        let map = HashMap::<String, serde_yaml::Value>::deserialize(de).map_err(|e| {
            serde::de::Error::custom(format!("assertion must be a single-key map: {e}"))
        })?;
        if map.len() != 1 {
            return Err(serde::de::Error::custom(
                "assertion must have exactly one key",
            ));
        }
        let (key, value) = map.into_iter().next().expect("single assertion");
        match key.as_str() {
            "exit_code" | "assert_exit_code" => convert::<i32>(value).map(SuiteAssertion::ExitCode),
            "process_not_crashed" | "assert_process_not_crashed" => {
                convert::<bool>(value).map(SuiteAssertion::ProcessNotCrashed)
            }
            other => Err(format!("unknown suite assertion: {other}")),
        }
        .map_err(serde::de::Error::custom)
    }
}
