# 05 — Test Definition DSL (YAML v1)

One portable format runnable from CLI, MCP (`tui_run_test`), and SDKs.

## 5.1 Minimal example (any language app)

```yaml
schema: tui-lab/v1
name: Navigation Test
application:
  command: "./myapp"
  args: []
  cwd: .
environment:
  TERM: xterm-256color
terminal:
  width: 120
  height: 40
setup:
  - wait_for_text: "Main Menu"
    timeout: 5s
steps:
  - press: ENTER
  - wait_for_text: "Dashboard"
  - assert_text:
      contains: "Dashboard"
cleanup:
  - press: q
assertions:
  - exit_code: 0
```

## 5.2 Full example (CodeGraph search, from source thread)

```yaml
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
  - wait_for_text: "CodeGraph"
  - press: ENTER
  - wait_for_text: "Projects"
  - press: "/"
  - type: "table"
  - press: ENTER
  - wait_for_text: "Search Results"
  - assert_text:
      contains: "table"
  - screenshot:
      name: "search-results"
  - press: q
assertions:
  - exit_code: 0
```

## 5.3 Step reference (v1)

| Step | Shape | Notes |
|------|-------|-------|
| `launch` | `{ command, args?, cwd?, env? }` | Usually implicit from `application:` |
| `press` | `"ENTER" \| "DOWN" \| "CTRL+C" \| "q"` | Named keys + literals |
| `press` (mouse) | `"CLICK x y" \| "RELEASE x y" \| "SCROLL_UP/DOWN x y"` | 1-based cells; gestures as separate steps (see `03` §3.3) |
| `type` | `"search text"` | Verbatim typing |
| `wait_for_text` | `{ text, timeout?, regex? }` | Polling wait — always prefer over `sleep` |
| `sleep` | `"500ms"` | Escape hatch only (flaky, discouraged) |
| `resize` | `{ width, height }` | e.g. 80x24, 120x40, 160x50, 40x15 |
| `assert_text` / `expect` | `{ contains/not_contains/regex/exact_text/cursor/exit_code/not_crashed/screen_changed/timeout }` | Polls to `timeout`; see `06` |
| `assert_region` | `{ x,y,width,height, contains }` | Cell-scoped; OOB names the grid size |
| `assert_exit_code` / `exit_code` | `0` | Post-cleanup |
| `snapshot` / `screenshot` | `{ name, mask?, styled?, graphics? }` | Golden-file compare (text default; styled = colors; graphics = Sixel) |
| `wait_for_exit` | `{ timeout }` | For quit flows |

Durations read as human strings: `500ms`, `3s`, `2m` (bare numbers mean milliseconds).

## 5.4 Suite selection (tags / skip / focus / sharding)

Suites carry optional selection metadata:

```yaml
tags: [smoke, login]   # --tags smoke runs suites carrying any listed tag
skip: true             # always reported as skipped, never fails
focus: true            # when any collected suite is focused, only focused run
```

CLI selection flags (`tuilab run --help`):

*   `--tags a,b` — run only suites carrying at least one listed tag.
*   `--shard N/M` — run the Nth 1-based slice of M (split CI matrices).
*   `--retries N` — rerun failures up to N extra times; the final attempt
    lands in the report with its `attempts` count.

Skipped suites report `skipped: true` (passing, counted separately);
filters matching nothing exit 0 with a message.

## 5.5 Other authoring paths (Phase 5, thin wrappers)

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
tui.type_text("table").await?;
tui.expect_text("Search Results").await?;
```

Python:

```python
from tuilab import TuiTest
tui = await TuiTest.launch(command="python app.py", width=120, height=40)
await tui.expect_text("Welcome")
await tui.press("ENTER")
await tui.expect_text("Dashboard")
```

> SDKs wrap the protocol — they never reimplement the engine. Start users on YAML + CLI.
