# 09 — SDK / Package Integration Guide

## 9.1 Rule: SDKs are thin wrappers (DECIDED: sidecar binary in v1)

**Decision:** Python/JS SDKs shell out to the `tuilab` sidecar binary over stdio/JSON in v1.
Zero native PTY code in SDKs; single behavior across CLI/MCP/SDK. Native bindings
(PyO3 / napi-rs) deferred to v3+ on demand.

```
Python SDK ----+
JS SDK --------+--> TUI Lab Test API / Protocol --> tui-lab-core
Rust SDK ------+
Java SDK(future)
```

Never reimplement PTY/emulator/assertions per language. SDKs serialize protocol actions and parse screens/results. This keeps behavior identical across CLI/MCP/SDK.

Embeddable core trait (Rust):

```rust
pub trait TuiRuntime {
    fn launch(&mut self, cmd: &str, width: u16, height: u16) -> anyhow::Result<SessionId>;
    fn press(&mut self, sess: &SessionId, key: Key) -> anyhow::Result<()>;
    fn type_text(&mut self, sess: &SessionId, text: &str) -> anyhow::Result<()>;
    fn screen(&self, sess: &SessionId) -> anyhow::Result<Screen>;
    fn resize(&mut self, sess: &SessionId, w: u16, h: u16) -> anyhow::Result<()>;
    fn close(&mut self, sess: &SessionId) -> anyhow::Result<ExitStatus>;
}
```

CLI, MCP, and all SDKs program against this.

## 9.2 Priority order (Phase 5)

1.  **Python** (`pip install tui-lab`) — covers Textual/Rich/Urwid.
2.  **JavaScript/TypeScript** (`@tui-lab/sdk`) — covers Ink/blessed.
3.  **Rust** (`tui-lab-sdk` crate) — covers Ratatui/Cursive/Crossterm.
4.  Later: Go, Java — only on demand; CLI already covers them.

## 9.3 Usage sketches

Python:

```python
from tuilab import TuiTest
tui = await TuiTest.launch(command="python app.py", width=120, height=40)
await tui.expect_text("Welcome")
await tui.press("ENTER")
await tui.expect_text("Dashboard")
```

TypeScript:

```ts
import { TuiTest } from "@tui-lab/sdk";
const tui = await TuiTest.launch({ command: "./myapp" });
await tui.expectText("Welcome");
await tui.press("ENTER");
await tui.type("/");
await tui.type("table");
await tui.press("ENTER");
await tui.expectText("Search Results");
```

Rust:

```rust
let tui = TuiTest::launch("./myapp").await?;
tui.expect_text("Welcome").await?;
tui.press(Key::Enter).await?;
```

YAML remains canonical: `await runner.run("test.yaml")` must work from every SDK.

## 9.4 Packaging notes

*   Python/JS SDKs ship as pure clients + JSON schema; they shell out to or import `tuilab` binary for execution (no native PTY code in SDK v1).
*   Rust SDK may link `tui-lab-core` directly for in-process speed.
*   All SDKs pin `schema: tui-lab/v1` and surface the same error/failure-bundle shape as CLI/MCP.

## 9.5 Future (Phase 6): optional instrumented metadata

Pure text testing can be fragile (`press ENTER` works, but `click "Submit"`
needs element identity terminals don't expose). Later, apps may OPT IN to
emitting test metadata — never required:

```json
{ "element": "submit_button", "role": "button", "label": "Submit",
  "bounds": [10, 5, 20, 1] }
```

Enabling framework-specific adapters (Ratatui / Bubble Tea / Textual / custom)
for semantic selectors, component trees, and focus state. Rule carried from the
source thread: generic black-box testing must always remain available;
adapters are enhancements, not requirements.
