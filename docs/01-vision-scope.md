# 01 — Vision & Scope

## 1.1 Problem

TUI apps (Ratatui, Crossterm, Cursive, Bubble Tea, Textual, Rich, Ink, ncurses, Lanterna, raw ANSI apps) fail in ways unit tests cannot catch:

*   wrong keyboard focus, ignored key events
*   screen not rendering / incorrect screen transition
*   ANSI escape sequence bugs, cursor position errors
*   terminal resize breakage, Unicode / wide-char rendering bugs
*   UI freeze, crash only in interactive mode
*   color / style regressions

Example flow we must automate (CodeGraph CLI example from source thread):

```
1. Launch app
2. Wait until "Projects" appears
3. Press Down, Press Enter
4. Verify project details appear
5. Press /, type "table", verify search results
6. Press Q, verify clean exit (code 0)
```

## 1.2 Product vision

**"Playwright for Terminal Applications"** — Record → Replay → Assert → Snapshot → Debug → CI, plus AI-assisted generation and agent-driven exploratory testing.

TUI Lab controls the **terminal environment**, not the app framework. Any process that accepts terminal input and emits terminal output is testable.

## 1.3 In scope

*   TUIs (fullscreen, alt-screen, mouse, resize-aware)
*   Interactive CLI wizards, installers, prompts
*   REPLs, DB shells, terminal dashboards
*   Shell menus, text games, terminal AI apps

## 1.4 Out of scope (v1)

*   Full terminal editors (vim/emacs) — possible later with special handling
*   tmux-nested apps — possible later
*   Framework-native component hooks — Phase 6 only, optional
*   Web dashboard / SaaS / cloud matrix — v3
*   Visual pixel-diff of styled output — v3 (v1 uses text + cell-JSON snapshots)

## 1.5 Users

*   TUI developers (Rust/Go/Python/Node/Java/C, Bash)
*   QA / CI maintainers
*   AI coding agents via MCP (Claude, ChatGPT, Cursor, IDE agents)

## 1.6 Three testing modes

| Mode | Requires app change? | How | Use |
|------|----------------------|-----|-----|
| 1. Black-box (default, v1) | No | PTY → terminal → assertions | Universal, all languages |
| 2. Instrumented (Phase 6) | Optional SDK | App exposes element tree / focus / events | Stable semantic selectors |
| 3. AI exploratory (via MCP) | No | Agent drives `tui_*` tools interactively | Bug repro, exploratory QA |

> Default must always work with zero app modification.

## 1.7 Terminal capability checklist

The engine reasons about terminal behavior, never frameworks. Coverage target:

*   Keyboard + mouse input, text rendering, ANSI colors/styles
*   Cursor position, screen buffer, alternate screen
*   Terminal resize, clipboard/paste, focus events
*   Signals, process lifecycle

v1 covers the first two lines; clipboard/focus/signals arrive in v2+.
