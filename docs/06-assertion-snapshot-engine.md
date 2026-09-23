# 06 — Assertion & Snapshot Engine

Assertions run against **terminal state** (grid + cursor + process), not raw bytes.

## 6.1 Taxonomy

```
- Text
  - contains / not_contains / regex / exact_text
- Screen
  - snapshot (golden compare) / region / dimensions / cursor_position
  - role (heuristic a11y widget: button / textinput / checkbox + name)
- Process
  - exit_code / crashed / timeout / stderr contains (future — child `stderr` shares the PTY stream; `portable-pty` 0.9 exposes no stderr redirect, see `11`)
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

Role (P5-E1, heuristic — inferred from TUI conventions `[ Label ]`,
`< Label >`, `[x]`, `Label: ___`; no app cooperation):

```yaml
- assert_text:
    role: button      # button | textinput | checkbox
    name: submit      # case-insensitive substring; omit to match any
    timeout: 2s       # optional poll, same as other asserts
```

`tuilab proto` / MCP: `{ "type": "role", "role": "button", "name": "submit" }`.
`tui_screen { tree: true }` dumps the same tree (`role/name/x/y/focused`)
for agent targeting. Custom-drawn widgets that ignore these conventions are
invisible to the tree — a documented limitation, not an error.

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

**Graphics (Sixel) snapshot** — captured DCS payloads, compared exact after
whitespace normalization (no tolerance: font-free rendering would flake
across machines). The grid crate retains no graphics, so this is structural
capture at feed time, not pixel rendering.

Storage (`snapshot: { name, styled?, graphics? }` selects the mode):

```
tests/
  snapshots/
    dashboard/
      boot/
        120x40.txt          # text (default)
        120x40.cells.json   # styled: true
        120x40.sixel.json   # graphics: true
```

Compare rule: `Expected != Actual → fail` with unified diff + rendered actual.

## 6.3 Determinism & masking

Volatile content (clock, CPU%, random IDs, spinners) must be maskable:

```yaml
- snapshot:
    name: dashboard
    mask:
      - regex: "\\d{2}:\\d{2}:\\d{2}"
      - region:bottom:3
```

Two mask layers, applied in order: `region:` entries blank rectangular
areas to spaces first (`region:rect:X,Y,W,H`, `region:top:N`,
`region:bottom:N` — 0-based, clamped, positions stay stable), then regex
entries replace volatile spans. Unknown `region:` names are errors.

Also support: `ignore_ansi_style: true`, per-cell `ignore_fg/bg`, custom normalizers.

## 6.4 Timing / retry semantics

*   Every `wait_*` / `expect_*`: `{ timeout = 10s default, poll_ms = 25-50 }`.
    Timeouts return on first match, so the default only costs time on genuine
    failures; tight defaults flaked on loaded CI (Phase 6).
*   Anti-pattern: `press ENTER` → immediate `assert`. Correct: `press ENTER` → `wait_for_text`.
*   On timeout: return last screen + elapsed + step index (feeds failure bundle in `11`).

## 6.5 Performance budgets (opt-in)

```yaml
budgets:
  step: 2s      # every step must complete within this
  suite: 60s    # the whole run must complete within this
  startup: 3s   # spawn-to-first-content must complete within this
```

Absent budgets never fail. Violations are ordinary test failures with
evidence — a slow suite is a result, not a broken harness (unlike
`terminal.timeout`, which kills hung runs as infra errors).
