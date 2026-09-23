//! SDK emitters for `record --target` (P5-C1).
//!
//! Pure string builders: recorded [`Step`]s become runnable Python, JS, or
//! Rust programs against the thin sidecars. Unsupported steps (assertions
//! the recorder never emits, sleeps) degrade to comments — emission never
//! fails, and the output always parses.

use tui_lab_protocol::Step;

/// Emission target for `record --target`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// `tui-lab/v1` YAML (default, existing behavior).
    Yaml,
    /// Python (`tui-lab` SDK).
    Python,
    /// JavaScript (`@tui-lab/sdk`).
    Js,
    /// Rust (`tui-lab-sdk`).
    Rust,
}

impl Target {
    /// Parse a `--target` value.
    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "yaml" => Ok(Self::Yaml),
            "python" | "py" => Ok(Self::Python),
            "js" | "javascript" => Ok(Self::Js),
            "rust" | "rs" => Ok(Self::Rust),
            other => Err(format!(
                "unknown --target {other:?}: expected yaml|python|js|rust"
            )),
        }
    }

    /// Output file extension for the target.
    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            Self::Yaml => "yaml",
            Self::Python => "py",
            Self::Js => "js",
            Self::Rust => "rs",
        }
    }
}

fn py_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn rs_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{{{:x}}}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Render recorded steps as a runnable program (YAML handled by serde
/// upstream — this covers the SDK targets).
pub fn render(target: Target, suite_name: &str, command: &str, steps: &[Step]) -> String {
    debug_assert_ne!(target, Target::Yaml);
    let mut lines = Vec::new();
    match target {
        Target::Yaml => unreachable!("YAML renders via serde"),
        Target::Python => {
            lines.push("import asyncio".to_string());
            lines.push("from tuilab import TuiTest".to_string());
            lines.push(String::new());
            lines.push(String::new());
            lines.push("async def main() -> None:".to_string());
            lines.push(format!(
                "    tui = await TuiTest.launch(command={})",
                py_string(command)
            ));
            lines.push("    try:".to_string());
            for step in steps {
                lines.push(format!("        {}", py_step(step)));
            }
            lines.push("    finally:".to_string());
            lines.push("        await tui.close()".to_string());
            lines.push(String::new());
            lines.push(String::new());
            lines.push("asyncio.run(main())".to_string());
        }
        Target::Js => {
            lines.push("import { TuiTest } from \"@tui-lab/sdk\";".to_string());
            lines.push(String::new());
            lines.push(format!(
                "const tui = await TuiTest.launch({});",
                js_string(command)
            ));
            lines.push("try {".to_string());
            for step in steps {
                lines.push(format!("  {}", js_step(step)));
            }
            lines.push("} finally {".to_string());
            lines.push("  await tui.close();".to_string());
            lines.push("}".to_string());
        }
        Target::Rust => {
            lines.push("use tui_lab_sdk::{LaunchOptions, TuiTest};".to_string());
            lines.push(String::new());
            lines.push("#[tokio::main]".to_string());
            lines.push("async fn main() -> Result<(), tui_lab_sdk::TuiLabError> {".to_string());
            lines.push(format!(
                "    let mut tui = TuiTest::launch({}, LaunchOptions::new()).await?;",
                rs_string(command)
            ));
            for step in steps {
                lines.push(format!("    {}", rs_step(step)));
            }
            lines.push("    tui.close(None, None).await?;".to_string());
            lines.push("    Ok(())".to_string());
            lines.push("}".to_string());
        }
    }
    let _ = suite_name;
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

fn js_string(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_string())
}

fn timeout_ms(timeout: Option<std::time::Duration>) -> u64 {
    timeout.map(|t| t.as_millis() as u64).unwrap_or(10_000)
}

fn py_step(step: &Step) -> String {
    match step {
        Step::Press(key) => format!("await tui.press({})", py_string(key)),
        Step::Type(text) => format!("await tui.type({})", py_string(text)),
        Step::WaitForText(wait) => format!(
            "await tui.expect_text({}, timeout_ms={})",
            py_string(&wait.text),
            timeout_ms(wait.timeout)
        ),
        Step::Sleep(sleep) => format!("# recorded sleep {:?} dropped — prefer waits", sleep.0),
        Step::Resize(to) => format!("await tui.resize({}, {})", to.width, to.height),
        Step::AssertText(assertion) => format!("# assert: {assertion:?}"),
        Step::Expect(assertion) => format!("# expect: {assertion:?}"),
        Step::AssertRegion(region) => format!("# assert_region: {region:?}"),
        Step::Snapshot(take) => format!("await tui.snapshot({})", py_string(&take.name)),
        Step::Screenshot(take) => format!("await tui.snapshot({})", py_string(&take.name)),
        Step::WaitForExit(wait) => {
            format!("# wait_for_exit {:?} dropped", wait.timeout)
        }
    }
}

fn js_step(step: &Step) -> String {
    match step {
        Step::Press(key) => format!("await tui.press({});", js_string(key)),
        Step::Type(text) => format!("await tui.type({});", js_string(text)),
        Step::WaitForText(wait) => format!(
            "await tui.expectText({}, {});",
            js_string(&wait.text),
            timeout_ms(wait.timeout)
        ),
        Step::Sleep(sleep) => format!("// recorded sleep {:?} dropped — prefer waits", sleep.0),
        Step::Resize(to) => format!("await tui.resize({}, {});", to.width, to.height),
        Step::AssertText(assertion) => format!("// assert: {assertion:?}"),
        Step::Expect(assertion) => format!("// expect: {assertion:?}"),
        Step::AssertRegion(region) => format!("// assert_region: {region:?}"),
        Step::Snapshot(take) => format!("await tui.snapshot({});", js_string(&take.name)),
        Step::Screenshot(take) => format!("await tui.snapshot({});", js_string(&take.name)),
        Step::WaitForExit(wait) => {
            format!("// wait_for_exit {:?} dropped", wait.timeout)
        }
    }
}

fn rs_step(step: &Step) -> String {
    match step {
        Step::Press(key) => format!("tui.press({}).await?;", rs_string(key)),
        Step::Type(text) => format!("tui.type_text({}, false).await?;", rs_string(text)),
        Step::WaitForText(wait) => format!(
            "tui.expect_text({}, {}, false, None).await?;",
            rs_string(&wait.text),
            timeout_ms(wait.timeout)
        ),
        Step::Sleep(sleep) => format!("// recorded sleep {:?} dropped — prefer waits", sleep.0),
        Step::Resize(to) => format!("tui.resize({}, {}).await?;", to.width, to.height),
        Step::AssertText(assertion) => format!("// assert: {assertion:?}"),
        Step::Expect(assertion) => format!("// expect: {assertion:?}"),
        Step::AssertRegion(region) => format!("// assert_region: {region:?}"),
        Step::Snapshot(take) => format!("tui.snapshot({}).await?;", rs_string(&take.name)),
        Step::Screenshot(take) => format!("tui.snapshot({}).await?;", rs_string(&take.name)),
        Step::WaitForExit(wait) => {
            format!("// wait_for_exit {:?} dropped", wait.timeout)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_steps() -> Vec<Step> {
        use tui_lab_protocol::{SnapshotTake, WaitForText};
        vec![
            Step::WaitForText(WaitForText {
                text: "Main".to_string(),
                regex: false,
                timeout: None,
            }),
            Step::Press("ENTER".to_string()),
            Step::Type("ta\"ble".to_string()),
            Step::Snapshot(SnapshotTake {
                name: "s".to_string(),
                mask: Vec::new(),
                styled: false,
                graphics: false,
            }),
        ]
    }

    #[test]
    fn targets_parse() {
        assert_eq!(Target::parse("yaml"), Ok(Target::Yaml));
        assert_eq!(Target::parse("py"), Ok(Target::Python));
        assert_eq!(Target::parse("javascript"), Ok(Target::Js));
        assert_eq!(Target::parse("rs"), Ok(Target::Rust));
        assert!(Target::parse("ruby").is_err());
    }

    #[test]
    fn python_renders_runnable_calls() {
        let out = render(Target::Python, "x", "./myapp", &sample_steps());
        assert!(
            out.contains("await TuiTest.launch(command=\"./myapp\")"),
            "{out}"
        );
        assert!(
            out.contains("await tui.expect_text(\"Main\", timeout_ms=10000)"),
            "{out}"
        );
        assert!(out.contains("await tui.press(\"ENTER\")"), "{out}");
        assert!(out.contains("await tui.type(\"ta\\\"ble\")"), "{out}");
        assert!(out.contains("await tui.snapshot(\"s\")"), "{out}");
        assert!(out.contains("await tui.close()"), "{out}");
    }

    #[test]
    fn js_renders_await_calls() {
        let out = render(Target::Js, "x", "./myapp", &sample_steps());
        assert!(out.contains("TuiTest.launch(\"./myapp\")"), "{out}");
        assert!(
            out.contains("await tui.expectText(\"Main\", 10000);"),
            "{out}"
        );
        assert!(out.contains("await tui.press(\"ENTER\");"), "{out}");
    }

    #[test]
    fn rust_renders_question_mark_calls() {
        let out = render(Target::Rust, "x", "./myapp", &sample_steps());
        assert!(
            out.contains("TuiTest::launch(\"./myapp\", LaunchOptions::new()).await?"),
            "{out}"
        );
        assert!(
            out.contains("tui.expect_text(\"Main\", 10000, false, None).await?;"),
            "{out}"
        );
        assert!(out.contains("tui.close(None, None).await?;"), "{out}");
    }
}
