# 13 — Feasibility & Risks

The product is **feasible**: it composes proven pieces (PTY controller + terminal emulator + test framework + snapshots + CI reporting). No research breakthrough needed. Hard part is not YAML/execution — it is emulation accuracy, PTY parity, sync, and debuggability.

## 13.1 Feasibility table

| Component | Feasibility | Difficulty |
|-----------|-------------|------------|
| CLI runner | Very high | Low |
| YAML protocol | Very high | Low |
| PTY runtime | High | Medium |
| Terminal emulator | High | **High** |
| Text assertions | Very high | Low |
| Snapshot testing | High | Medium |
| Cross-platform (ConPTY) | High | **High** |
| MCP server | Very high | Low–Medium |
| Python / JS SDK | Very high | Low |
| Framework adapters | Medium | Medium–High |
| Semantic element tree | Medium | High |
| AI autonomous testing | High | Medium |
| Cloud execution | High | Medium |

Framework coverage is inherently good because testing is at the terminal-protocol level (no per-framework integration):

| Framework | Feasibility |
|-----------|-------------|
| Rust Ratatui / Cursive / Crossterm | Excellent |
| Go Bubble Tea | Excellent |
| Python Textual | Excellent |
| Python Rich | Good |
| ncurses | Good |
| Java Lanterna | Good |
| Node Ink | Good |
| Bash interactive apps | Good |
| Vim/Neovim plugins | Possible, special handling |
| tmux-based apps | Possible |
| Full terminal editors | Advanced |

## 13.2 Top risks + mitigations

**1. Terminal emulator accuracy (High).** CSI/OSC/SGR, cursor, alt-screen, scroll regions, insert/delete, Unicode width, combining chars, mouse, bracketed paste, focus events. Basic parser passes simple TUIs, fails advanced ones.
→ Mitigation: use `vte` for parsing + custom grid; golden corpus of recordings per framework; fuzz against `xterm` behavior.

**2. Timing / async rendering (Medium).** Screen lags input; naive assert flakes.
→ Mitigation: `wait_for_text` + polling everywhere; ban `sleep` in recorder output; per-step timeouts in failure bundle.

**3. Determinism (Medium).** Clocks, CPU%, IDs, spinners break snapshots.
→ Mitigation: regex/region masks, style-ignore flags, normalizers (see `06`).

**4. Cross-platform PTY (High).** ConPTY ≠ Unix PTY (chodzi o resize, CTRL+C, alt-screen, encoding).
→ Mitigation: `portable-pty` abstraction, dual-OS CI matrix, Windows-specific test suite (see `12`).

**5. Complex keyboard sequences (Medium).** CTRL/ALT/F-keys/mouse encode differently per terminal.
→ Mitigation: named-key table + integration tests per sequence; log raw bytes sent in `--debug`.
