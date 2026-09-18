# 06 — Assertion & Snapshot Engine

Assertions run against **terminal state** (grid + cursor + process), not raw bytes.

## 6.1 Taxonomy

```
- Text
  - contains / not_contains / regex / exact_text
- Screen
  - snapshot (golden compare) / region / dimensions / cursor_position
- Process
  - exit_code / crashed / timeout / stderr contains
- Interaction
  - key accepted (screen changed within N ms) / screen changed / focus changed
- Performance
  - startup_time / render_time / response_latency
```

YAML:

```yaml
- expect:
    text: "Dashboard"
- expect:
    not_text: "Unhandled panic"
- expect:
    screen_changed: true
- expect:
    startup_time_less_than: 2s
- assert_region:
    x: 0
    y: 0
    width: 80
    height: 5
    contains: "CodeGraph"
- assert:
    text_visible: "Dashboard"
```

Cursor:

```yaml
assertions:
  - cursor_position:
      row: 3
      col: 10
```

Process:

```yaml
- assert_exit_code: 0
- assert_process_not_crashed: true
```

## 6.2 Snapshots (two kinds)

**Text snapshot** — plain grid join. Good for deterministic layout.

```
Dashboard
---------
CPU: 20%
RAM: 40%
```

**Render (cell) snapshot** — JSON with style:

```json
{ "width": 80, "height": 24,
  "cells": [ { "x": 0, "y": 0, "char": "D", "fg": "white", "bg": "black" } ] }
```

Detects: colors, bold/underline/reverse, cursor, Unicode width, box-drawing.

Storage:

```
tests/
  snapshots/
    dashboard/
      80x24.json
      120x40.json
```

Compare rule: `Expected != Actual → fail` with unified diff + rendered actual.

## 6.3 Determinism & masking

Volatile content (clock, CPU%, random IDs, spinners) must be maskable:

```yaml
- snapshot:
    name: dashboard
    mask:
      - regex: "\\d{2}:\\d{2}:\\d{2}"
      - region: bottom_status
```

Also support: `ignore_ansi_style: true`, per-cell `ignore_fg/bg`, custom normalizers.

## 6.4 Timing / retry semantics

*   Every `wait_*` / `expect_*`: `{ timeout = 10s default, poll_ms = 25-50 }`.
    Timeouts return on first match, so the default only costs time on genuine
    failures; tight defaults flaked on loaded CI (Phase 6).
*   Anti-pattern: `press ENTER` → immediate `assert`. Correct: `press ENTER` → `wait_for_text`.
*   On timeout: return last screen + elapsed + step index (feeds failure bundle in `11`).
