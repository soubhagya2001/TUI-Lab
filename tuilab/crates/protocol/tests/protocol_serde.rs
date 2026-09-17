//! Protocol wire conformance: JSON actions + YAML test files.

use tui_lab_protocol::{Action, Step, TestFile};

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
fn unsupported_schema_version_is_rejected() {
    let yaml = "schema: tui-lab/v9\nname: x\napplication:\n  command: y\nsteps: []\n";
    assert!(TestFile::from_yaml(yaml).is_err());
}

#[test]
fn malformed_yaml_is_rejected() {
    assert!(TestFile::from_yaml("schema: [unclosed").is_err());
}
