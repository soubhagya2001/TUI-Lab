# 03 — Terminal Runtime (PTY + Emulator)

This is the heart of TUI Lab. Accuracy here determines product quality.

## 3.1 Why PTY (not plain subprocess)?

Plain `Command::new("./myapp").output()` fails because TUIs probe:

*   `isatty(stdin/stdout)?`
*   terminal size (`SIGWINCH` / `GetConsoleScreenBufferInfo`)
*   raw mode support, `$TERM` type, color capability
*   alternate screen, bracketed paste, focus events

A pseudo-terminal makes the app believe it runs in a real terminal.

```
Test Engine
     |
     v
PTY Master (controller)
     |-- stdin  -> TUI Application
     |-- stdout <- TUI Application
     +-- resize (rows/cols)
              |
              v
         TUI Process
```

Enables: raw keyboard, alt-screen, cursor moves, ANSI, resize, prompts, fullscreen redraw, mouse, color — and later: clipboard/paste, focus events, signals (see capability checklist in `01-vision-scope.md` testing modes).

## 3.2 Terminal emulator layer (DECIDED: reuse grid crate)

**Decision:** reuse an existing terminal-grid crate (e.g. `alacritty_terminal`) for grid/state
machine; keep only PTY management, input encoding, and the protocol/assertion layers custom.
Rationale: emulator accuracy is the #1 risk (see `13-feasibility-risks.md`); a battle-tested
grid cuts CSI/OSC/SGR/scroll-region/alt-screen bug surface vs a fully custom buffer.

Must interpret e.g.:

```
ESC[2J      clear screen
ESC[H       home cursor
ESC[1;1H    set cursor
ESC[31m     fg red
ESC[0m      reset style
```

Plus: CSI, OSC, SGR colors (16/256/truecolor), cursor save/restore, scroll regions, insert/delete lines, line wrap, alt-screen enter/exit, bracketed paste, focus in/out, mouse (SGR/X10), Unicode width.

### Virtual screen model

```rust
pub struct TerminalBuffer {
    pub width: u16,
    pub height: u16,
    pub cells: Vec<Cell>,   // width * height, row-major
    pub cursor: Cursor,     // row, col, visible, shape
    pub alternate_screen: bool,
    pub scrollback: Vec<Row>,
}

pub struct Cell {
    pub character: char,
    pub foreground: Color,
    pub background: Color,
    pub bold: bool,
    pub underline: bool,
    pub reverse: bool,
}
```

Example: output `"Hello" + ESC[2;5H + "World"` →
row 0: `Hello`, row 1 col 4: `World`. Tests inspect this grid, not raw bytes.

## 3.3 Input injection

*   Printable chars → bytes (UTF-8)
*   Named keys → escape sequences: `ENTER (\r)`, `ESC`, `UP/DOWN/LEFT/RIGHT`, `TAB/BACKTAB`, `HOME/END/PGUP/PGDN/INSERT/DELETE`, `F1-F12`, `CTRL+C/A/...`, `ALT+x`
*   Mouse (v2): click/drag/scroll encoded as SGR mouse sequences
*   Resize: `pty.resize(cols, rows)` + optional `SIGWINCH` propagation
*   Paste: bracketed-paste wrap when enabled

## 3.4 Process management

*   Spawn with `cwd`, `args`, `env` (`TERM=xterm-256color` default, override per test)
*   Track PID, exit code, signal, `stderr` capture (pipe, not PTY, to separate app logs from screen)
*   Kill tree on timeout; grace period (`SIGTERM` → `SIGKILL` / Win `TerminateProcess`)
*   Crash detection: non-zero exit, signal death, panic string on screen/stderr

## 3.5 Timing model

TUI redraw is async. Never assert immediately after input without a wait primitive. Core loop:

```
write input -> poll screen buffer every 10-50ms until condition or timeout
```

See `04-test-protocol-spec.md` (`wait_for_text`) and `06-assertion-snapshot-engine.md`.
