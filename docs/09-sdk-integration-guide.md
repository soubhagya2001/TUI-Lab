# 09 — SDK / Package Integration Guide

## 9.1 Rule: SDKs are thin wrappers (DECIDED: sidecar binary in v1)

**Decision:** Python/JS SDKs shell out to the `tuilab` sidecar binary over stdio/JSON in v1.
Zero native PTY code in SDKs; single behavior across CLI/MCP/SDK. Native bindings
(PyO3 / napi-rs) deferred to v3+ on demand.

### 9.1.1 The `tuilab proto` wire contract (Phase 5)

Interactive SDKs speak `tuilab proto`: one JSON action per stdin line (the
`docs/04` shapes, validated by the shared field table), one JSON response
per stdout line, logs on stderr. Success is `{"ok": true, ...fields}`;
failures are `{"ok": false, "error": "..."}`. EOF closes the engine.

```json
{"action": "launch", "command": "python app.py", "terminal": {"width": 120, "height": 40}}
{"action": "press", "session_id": "sess_001", "key": "ENTER"}
{"action": "wait_for_text", "session_id": "sess_001", "text": "Dashboard"}
{"action": "close", "session_id": "sess_001"}
```

The contract is pinned by `tuilab/crates/cli/tests/proto_roundtrip.rs`
(all 9 shapes) — an engine change that breaks SDKs fails there first.

### 9.1.2 Binary resolution and errors (Python SDK)

`find_binary()` order: explicit argument → `TUILAB_BIN` env → `PATH` →
workspace `target/debug` layout. Engine errors surface as `TuiLabError`
carrying the reply dict (step/session context intact); transport breakdowns
(non-JSON, EOF, timeouts) raise the same type. `type(..., sensitive=True)`
never logs the text. `Runner.run("test.yaml")` shells `tuilab run` and
returns parsed `reports/results.json`, raising on infra exit codes (2–4).

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

1.  **Python** (`pip install tui-lab`) — covers Textual/Rich/Urwid. DONE:
    `tuilab/sdks/python` (`TuiTest` async API + `Runner.run`), sidecar over
    `tuilab proto`, pytest suite green on Windows (Linux via CI).
2.  **JavaScript/TypeScript** (`@tui-lab/sdk`) — covers Ink/blessed. DONE:
    `tuilab/sdks/javascript` (typed `TuiTest` + `Runner`, `node:test`
    suite, `npm run build && npm test`), same sidecar contract, mirrored
    fixture flow green on Windows (Linux via CI).
3.  **Rust** (`tui-lab-sdk` crate) — covers Ratatui/Cursive/Crossterm. DONE:
    `tuilab/sdks/rust` workspace member (same surface in `Result`-based
    async API, `tokio` integration tests), sidecar only — no core linking
    per DECIDED (see `src/lib.rs` crate docs).
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
const tui = await TuiTest.launch("./myapp");
try {
  await tui.expectText("Welcome");
  await tui.press("ENTER");
  await tui.type("/");
  await tui.type("table");
  await tui.press("ENTER");
  await tui.expectText("Search Results");
} finally {
  await tui.close();
}
```

Rust:

```rust
let mut tui = tui_lab_sdk::TuiTest::launch("./myapp", tui_lab_sdk::LaunchOptions::new()).await?;
tui.expect_text("Welcome", 10_000, false).await?;
tui.press("ENTER").await?;
tui.close().await?;
```

YAML remains canonical: `await runner.run("test.yaml")` must work from every SDK.

## 9.4 Packaging notes

*   Python/JS SDKs ship as pure clients + JSON schema; they shell out to the `tuilab` binary for execution (no native PTY code in SDK v1).
*   Rust SDK likewise shells out (sidecar rule upheld over the §9.1 sketch's native-link option — overturning needs a grill round).
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
