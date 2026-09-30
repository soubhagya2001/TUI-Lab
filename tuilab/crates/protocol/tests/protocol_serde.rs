//! Protocol wire conformance: JSON actions + YAML test files.

use tui_lab_protocol::{parse_duration, Action, Step, TestFile};

// --- JSON actions (docs/04 §4.2) ---

#[test]
fn launch_round_trip_with_defaults() {
    let json = r#"{"action":"launch","command":"python app.py"}"#;
    let action = Action::from_json(json).expect("parse launch");
    assert_eq!(action.name(), "launch");
    let back = action.to_json().expect("serialize");
    let again = Action::from_json(&back).expect("re-parse");
    assert_eq!(action, again);
    match again {
        Action::Launch {
            args, timeout_ms, ..
        } => {
            assert!(args.is_empty());
            assert_eq!(timeout_ms, 10_000);
        }
        other => panic!("expected launch, got {other:?}"),
    }
}

#[test]
fn press_type_wait_screen_assert_snapshot_resize_close_parse() {
    let sid = "sess_123";
    let cases = [
        format!(r#"{{"action":"press","session_id":"{sid}","key":"ENTER"}}"#),
        format!(r#"{{"action":"type","session_id":"{sid}","text":"hi"}}"#),
        format!(r#"{{"action":"wait_for_text","session_id":"{sid}","text":"Dashboard"}}"#),
        format!(r#"{{"action":"screen","session_id":"{sid}"}}"#),
        format!(
            r#"{{"action":"assert","session_id":"{sid}","condition":{{"type":"text_visible","value":"x"}}}}"#
        ),
        format!(r#"{{"action":"snapshot","session_id":"{sid}","name":"dash"}}"#),
        format!(r#"{{"action":"resize","session_id":"{sid}","width":80,"height":24}}"#),
        format!(r#"{{"action":"close","session_id":"{sid}"}}"#),
    ];
    let names = [
        "press",
        "type",
        "wait_for_text",
        "screen",
        "assert",
        "snapshot",
        "resize",
        "close",
    ];
    for (json, name) in cases.iter().zip(names) {
        let action = Action::from_json(json).expect(name);
        assert_eq!(action.name(), name);
    }
}

#[test]
fn wait_defaults_apply() {
    let action = Action::from_json(r#"{"action":"wait_for_text","session_id":"s","text":"x"}"#)
        .expect("parse wait");
    match action {
        Action::WaitForText {
            timeout_ms,
            poll_ms,
            regex,
            ..
        } => {
            assert_eq!(timeout_ms, 3_000);
            assert_eq!(poll_ms, 50);
            assert!(!regex);
        }
        other => panic!("expected wait_for_text, got {other:?}"),
    }
}

#[test]
fn unknown_action_is_rejected() {
    assert!(Action::from_json(r#"{"action":"dance"}"#).is_err());
}

#[test]
fn unknown_fields_are_rejected() {
    assert!(
        Action::from_json(r#"{"action":"press","session_id":"s","key":"ENTER","bogus":1}"#)
            .is_err()
    );
}

// --- YAML test files (docs/05) ---

const FULL_EXAMPLE: &str = r#"
schema: tui-lab/v1
name: CodeGraph Project Search
application:
  command: "./codegraph"
  args: ["--project", "./sample-project"]
terminal:
  width: 120
  height: 40
  timeout: 10s
steps:
  - wait_for_text:
      text: "CodeGraph"
  - press: ENTER
  - wait_for_text:
      text: "Projects"
  - press: "/"
  - type: "table"
  - press: ENTER
  - wait_for_text:
      text: "Search Results"
  - assert_text:
      contains: "table"
  - screenshot:
      name: "search-results"
  - press: q
assertions:
  - exit_code: 0
"#;

#[test]
fn full_example_parses() {
    let file = TestFile::from_yaml(FULL_EXAMPLE).expect("parse full example");
    assert_eq!(file.schema, "tui-lab/v1");
    assert_eq!(file.name, "CodeGraph Project Search");
    assert_eq!(file.application.command, "./codegraph");
    assert_eq!(file.application.args.len(), 2);
    assert_eq!(file.terminal.width, 120);
    assert_eq!(file.terminal.height, 40);
    assert_eq!(file.steps.len(), 10);
    assert!(matches!(file.steps[0], Step::WaitForText(_)));
    assert!(matches!(file.steps[1], Step::Press(_)));
    assert!(matches!(file.steps[4], Step::Type(_)));
    assert!(matches!(file.steps[8], Step::Screenshot(_)));
    assert_eq!(file.assertions.len(), 1);
}

#[test]
fn minimal_example_with_setup_cleanup_parses() {
    let yaml = r#"
schema: tui-lab/v1
name: Navigation Test
application:
  command: "./myapp"
environment:
  TERM: xterm-256color
terminal:
  width: 120
  height: 40
setup:
  - wait_for_text:
      text: "Main Menu"
      timeout: 5s
steps:
  - press: ENTER
  - wait_for_text:
      text: "Dashboard"
  - assert_text:
      contains: "Dashboard"
cleanup:
  - press: q
assertions:
  - exit_code: 0
"#;
    let file = TestFile::from_yaml(yaml).expect("parse minimal");
    assert_eq!(file.setup.len(), 1);
    assert_eq!(file.cleanup.len(), 1);
    match &file.setup[0] {
        Step::WaitForText(wait) => {
            assert_eq!(wait.text, "Main Menu");
            assert_eq!(wait.timeout, Some(std::time::Duration::from_secs(5)));
        }
        other => panic!("expected wait_for_text, got {other:?}"),
    }
}

#[test]
fn scalar_wait_for_text_parses() {
    let yaml = r#"
schema: tui-lab/v1
name: Scalar wait
application:
  command: "./myapp"
steps:
- wait_for_text: "Ready"
"#;
    let file = TestFile::from_yaml(yaml).expect("parse scalar wait");
    match &file.steps[0] {
        Step::WaitForText(wait) => {
            assert_eq!(wait.text, "Ready");
            assert!(!wait.regex);
            assert!(wait.timeout.is_none());
        }
        other => panic!("expected wait_for_text, got {other:?}"),
    }
}

#[test]
fn unsupported_schema_version_is_rejected() {
    let yaml = "schema: tui-lab/v9\nname: x\napplication:\n  command: y\nsteps: []\n";
    assert!(TestFile::from_yaml(yaml).is_err());
}

#[test]
fn malformed_yaml_is_rejected() {
    assert!(TestFile::from_yaml("schema: [unclosed").is_err());
}

#[test]
fn tags_skip_focus_parse_with_defaults() {
    // Phase A: selection metadata is optional and defaults to run-everything.
    let plain =
        TestFile::from_yaml("schema: tui-lab/v1\nname: x\napplication:\n  command: y\nsteps: []\n")
            .expect("parse");
    assert!(plain.tags.is_empty());
    assert!(!plain.skip);
    assert!(!plain.focus);
    let tagged = TestFile::from_yaml(
        "schema: tui-lab/v1\nname: x\napplication:\n  command: y\ntags:\n  - smoke\n  - login\nskip: true\nfocus: true\nsteps: []\n",
    )
    .expect("parse");
    assert_eq!(tagged.tags, ["smoke", "login"]);
    assert!(tagged.skip);
    assert!(tagged.focus);
}

#[test]
fn empty_step_and_assertion_maps_are_errors_not_panics() {
    // R11: single-key extraction must never panic on empty maps.
    let yaml = "schema: tui-lab/v1\nname: x\napplication:\n  command: y\nsteps:\n  - {}\n";
    assert!(TestFile::from_yaml(yaml).is_err());
    let yaml = "schema: tui-lab/v1\nname: x\napplication:\n  command: y\nsteps:\n  - press: q\nassertions:\n  - {}\n";
    assert!(TestFile::from_yaml(yaml).is_err());
}

#[test]
fn durations_parse_units_and_reject_overflow() {
    // R4: minutes use checked math; bare numbers mean milliseconds.
    use std::time::Duration;
    assert_eq!(parse_duration("500ms"), Ok(Duration::from_millis(500)));
    assert_eq!(parse_duration("500"), Ok(Duration::from_millis(500)));
    assert_eq!(parse_duration("3s"), Ok(Duration::from_secs(3)));
    assert_eq!(parse_duration("2m"), Ok(Duration::from_secs(120)));
    assert!(parse_duration("forever").is_err());
    assert!(parse_duration("10h").is_err());
    // u64::MAX minutes overflows seconds — must error, never wrap.
    assert!(parse_duration("18446744073709551615m").is_err());
}

// --- P5-E2: timing + scalar/map press/type ---

#[test]
fn press_type_scalar_and_map_forms_round_trip() {
    let yaml = r#"
schema: tui-lab/v1
name: pacing
application:
  command: ./app
timing:
  key_delay: 40ms
  input_delay: 10ms
steps:
  - press: ENTER
  - press:
      key: DOWN
      delay: 100ms
  - type: hello
  - type:
      text: world
      delay: 50ms
assertions: []
"#;
    let file = TestFile::from_yaml(yaml).expect("parse timing suite");
    assert_eq!(
        file.timing.key_delay,
        Some(std::time::Duration::from_millis(40))
    );
    assert_eq!(
        file.timing.input_delay,
        Some(std::time::Duration::from_millis(10))
    );
    match &file.steps[0] {
        Step::Press(p) => {
            assert_eq!(p.key, "ENTER");
            assert_eq!(p.delay, None);
        }
        other => panic!("expected press scalar, got {other:?}"),
    }
    match &file.steps[1] {
        Step::Press(p) => {
            assert_eq!(p.key, "DOWN");
            assert_eq!(p.delay, Some(std::time::Duration::from_millis(100)));
        }
        other => panic!("expected press map, got {other:?}"),
    }
    match &file.steps[2] {
        Step::Type(t) => {
            assert_eq!(t.text, "hello");
            assert_eq!(t.delay, None);
        }
        other => panic!("expected type scalar, got {other:?}"),
    }
    match &file.steps[3] {
        Step::Type(t) => {
            assert_eq!(t.text, "world");
            assert_eq!(t.delay, Some(std::time::Duration::from_millis(50)));
        }
        other => panic!("expected type map, got {other:?}"),
    }
    // Serialize straight back: scalar forms stay scalar (no delay → no map).
    let back = serde_yaml::to_string(&file).expect("re-serialize");
    assert!(
        back.contains("- press: ENTER"),
        "scalar press kept:\n{back}"
    );
    assert!(back.contains("- type: hello"), "scalar type kept:\n{back}");
    assert!(back.contains("key: DOWN"), "map press kept:\n{back}");
    assert!(back.contains("text: world"), "map type kept:\n{back}");
    assert!(back.contains("key_delay: 40ms"), "timing kept:\n{back}");
    let again = TestFile::from_yaml(&back).expect("re-parse round trip");
    assert_eq!(again, file);
}

#[test]
fn press_with_unknown_map_fields_is_an_error() {
    let yaml = r#"
schema: tui-lab/v1
name: bad
application:
  command: ./app
steps:
  - press:
      key: ENTER
      bogus: 1
"#;
    assert!(TestFile::from_yaml(yaml).is_err());
}

#[test]
fn action_press_type_delay_ms_round_trip() {
    let sid = "sess_e2";
    let press = Action::from_json(&format!(
        r#"{{"action":"press","session_id":"{sid}","key":"ENTER","delay_ms":25}}"#
    ))
    .expect("parse press delay");
    match press {
        Action::Press {
            delay_ms, ref key, ..
        } => {
            assert_eq!(key, "ENTER");
            assert_eq!(delay_ms, Some(25));
        }
        other => panic!("expected press, got {other:?}"),
    }
    let back = press.to_json().expect("serialize");
    let again = Action::from_json(&back).expect("re-parse");
    assert_eq!(press, again);

    let typed = Action::from_json(&format!(
        r#"{{"action":"type","session_id":"{sid}","text":"hi","sensitive":false,"delay_ms":10}}"#
    ))
    .expect("parse type delay");
    match typed {
        Action::Type { delay_ms, text, .. } => {
            assert_eq!(text, "hi");
            assert_eq!(delay_ms, Some(10));
        }
        other => panic!("expected type, got {other:?}"),
    }
}
