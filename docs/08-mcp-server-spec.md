# 08 — MCP Server Spec (`tuilab-mcp`)

Expose TUI Lab as MCP tools over stdio so AI agents (Claude / ChatGPT / Cursor / IDEs) can test any terminal app.

```
MCP Client (agent) -> TUI Lab MCP Server (stdio) -> tui-lab-core -> App PTY
```

Implementation (Phase 4): `rmcp` 3.x (`server`, `macros`, `schemars`,
`transport-io` features), `#[tool_router]` + `#[tool]` methods over a shared
`SessionRegistry` in `tui-lab-core`. Tracing goes to stderr; stdout belongs
to the transport. Scope is tools only — no prompts/resources.

## 8.1 Tool list (9 tools, small + powerful)

| Tool | Input | Returns |
|------|-------|---------|
| `tui_launch` | `{ command, args?, cwd?, width?, height?, env? }` | `{ session_id, status, screen }` |
| `tui_press` | `{ session_id, key }` | `{ ok, screen_changed }` |
| `tui_type` | `{ session_id, text }` | `{ ok }` |
| `tui_screen` | `{ session_id, style? }` | `{ width, height, cursor, text, cells? }` |
| `tui_wait_for_text` | `{ session_id, text, timeout_ms?, regex? }` | `{ found, elapsed_ms, screen }` |
| `tui_assert` | `{ session_id, assertion }` | `{ passed, detail }` |
| `tui_snapshot` | `{ session_id, name }` | `{ saved, diff? }` |
| `tui_run_test` | `{ test_file, terminal? }` | `{ status, passed, failed, failures[] }` |
| `tui_close` | `{ session_id, quit? }` | `{ success, exit_code, signal }` |

Phase 4 field notes (deviations from early sketches, kept honest):

*   `tui_close` returns `success` + `exit_code` + `signal` (numeric codes
    landed in Phase 9c via `portable-pty` `exit_code()`; signal deaths read
    code per PTY convention). Pass `quit` (e.g. `"q"`) so close polls for
    natural exit within the grace period and only then kills — this removes
    the press-quit/close race on slow schedulers.
*   `tui_screen { styled: true }` returns the per-cell array (`x/y/char/
    fg/bg/bold/underline/reverse`, deterministic color encoding); text +
    cursor + dims always return.
*   `tui_snapshot` returns `{ saved: true }` when it writes a new golden,
    `{ saved: false, diff }` when it compares.
*   Allowlist patterns are regexes: `^\./.*`, `^cargo run.*`, `^python.*`
    (see `tuilab.example.yaml`).

Example:

```json
// tui_launch
{ "command": "./myapp", "args": [], "cwd": "./project", "width": 120, "height": 40 }
// -> { "session_id": "sess_123", "status": "running", "screen": "Welcome to MyApp" }

// tui_press
{ "session_id": "sess_123", "key": "ENTER" }

// tui_screen -> { "width": 120, "height": 40,
//   "cursor": { "row": 2, "col": 5 }, "text": "Welcome..." }
```

## 8.2 Mode A — Interactive agent testing

```
Launch -> Inspect screen -> Press key -> Inspect -> Type -> Assert
```

Use for: debugging, exploratory QA, bug repro, interactive dev.

Example agent trace:

```
AI: tui_launch("./target/debug/myapp")
    -> screen: [My Application, > Start, Settings, Exit]
AI: tui_press("ENTER")
AI: tui_screen()
    -> screen: [Running, Status: OK]
AI: "Navigation from main menu to running state works."
```

## 8.3 Mode B — Automated execution

Agent calls `tui_run_test`, engine runs full YAML:

```json
{ "test_file": "tests/login.yaml" }
// -> { "status": "failed", "passed": 8, "failed": 1,
//      "failures": [{ "test": "search", "step": 4,
//                     "expected": "Results", "actual": "No matches" }] }
```

Prefer Mode B for regression; Mode A for investigation. Do not make the agent press every key when a YAML suite exists.

## 8.4 Session & safety rules (DECIDED: allowlist + cwd jail)

*   One PTY per `session_id`; idle timeout (default 60s) auto-closes orphans.
*   Max concurrent sessions configurable (default 8); per-cwd sandbox.
*   **Command allowlist:** `tui_launch` only runs command lines matching `security.allow_commands` regex list in `tuilab.yaml` (defaults: `^\./.*`, `^cargo run.*`, `^python.*`). Everything else rejected with `FORBIDDEN_COMMAND`.
*   **Cwd jail:** `cwd` must resolve under project root; `..` escape rejected.
*   `tui_close` always kills process tree; no leaked ConPTY/PTY handles.
*   Never echo secrets: `tui_type` with `sensitive: true` redacts from logs.
*   Tool descriptions must instruct agents to use `wait_for_text` after every input.
