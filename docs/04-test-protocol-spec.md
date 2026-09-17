# 04 — Test Protocol Spec (v1, session-based)

The protocol is the product foundation. CLI, MCP, SDKs, and future HTTP all speak it. Language-independent (JSON over stdio / function calls / files).

## 4.1 Session lifecycle

```
Create Session -> Launch App -> session_id
  |- Send Input (press / type / resize / signal)
  |- Read Screen (screen / wait_for_text)
  |- Assert / Snapshot
  +- Close (exit code, cleanup)
```

`session_id` (e.g. `sess_abc123`) preserves app state across calls — required for MCP agents and parallel tests.

## 4.2 Actions (v1)

### launch

```json
{ "action": "launch", "command": "python app.py", "args": [],
  "cwd": "./project", "terminal": { "width": 120, "height": 40 },
  "env": { "TERM": "xterm-256color" }, "timeout_ms": 10000 }
```

Returns: `{ "session_id": "sess_123", "status": "running", "screen": "Welcome..." }`

### press

```json
{ "action": "press", "session_id": "sess_123", "key": "ENTER" }
```

Keys: `ENTER ESC TAB BACKTAB UP DOWN LEFT RIGHT HOME END PGUP PGDN INSERT DELETE F1..F12`, `CTRL+C`, `CTRL+D`, single chars, `ALT+x`. Case-insensitive aliases (`DOWN` = `Down`).

### type

```json
{ "action": "type", "session_id": "sess_123", "text": "hello" }
```

Types verbatim (no Enter appended unless `\r` included).

### wait_for_text

```json
{ "action": "wait_for_text", "session_id": "sess_123",
  "text": "Dashboard", "timeout_ms": 3000, "poll_ms": 50 }
```

Polls screen buffer until substring/regex appears. Prefer over `sleep`.

### screen

```json
{ "action": "screen", "session_id": "sess_123" }
```

Returns:

```json
{ "width": 120, "height": 40,
  "cursor": { "row": 2, "col": 5, "visible": true },
  "text": "Welcome...\n> Start\n  Settings",
  "cells": [ { "x": 0, "y": 0, "char": "W", "fg": "white", "bg": "black" } ] }
```

`text` = plain-text grid join; `cells` optional (for color/style assertions, snapshots).

### assert

```json
{ "action": "assert", "session_id": "sess_123",
  "condition": { "type": "text_visible", "value": "Dashboard" } }
```

Full taxonomy in `06-assertion-snapshot-engine.md`.

### snapshot

```json
{ "action": "snapshot", "session_id": "sess_123", "name": "dashboard" }
```

Captures text + cell-JSON; compares to `tests/snapshots/<name>/<WxH>.json` with masks.

### resize

```json
{ "action": "resize", "session_id": "sess_123", "width": 80, "height": 24 }
```

Follow with `wait_for_text` — redraw is async.

### close

```json
{ "action": "close", "session_id": "sess_123", "signal": "q", "timeout_ms": 2000 }
```

Sends quit input (or SIGTERM), waits for exit, returns `{ "exit_code": 0, "crashed": false }`.

## 4.3 Transports

| Transport | Use | Mapping |
|-----------|-----|---------|
| CLI (`tuilab run`) | Local dev/CI | YAML steps → protocol calls in-process |
| MCP stdio | AI agents | `tui_*` tools → same protocol handlers (see `08`) |
| SDK calls | In-language tests | `TuiTest.launch()/press()` → protocol (see `09`) |
| HTTP/WS (future) | Remote agents, live streaming | REST + WS `screen` frames |

## 4.4 Versioning

*   `version: "1.0"` in every test file; unknown fields ignored with warning.
*   Additive changes only within v1. Breaking changes → `tui-lab/v2` + migration note.
