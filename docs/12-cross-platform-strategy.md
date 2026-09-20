# 12 — Cross-Platform Strategy (Windows-first)

TUI Lab must be first-class on Windows (primary dev OS per source) plus Linux/macOS.

## 12.1 PTY abstraction

| OS | Mechanism | Notes |
|----|-----------|-------|
| Linux/macOS | Unix PTY (`fork`/`exec`, `openpty`) | `SIGWINCH` for resize, termios raw mode |
| Windows 10+ | ConPTY (`CreatePseudoConsole`) | Pipes for in/out, resize via `ResizePseudoConsole` |

Rust crate: **`portable-pty`** — single API over both. Evaluate early; wrap in `tui-lab-pty` crate so engine never branches on OS except in that crate.

```
TUI Lab App
  -> ConPTY Controller (Windows) / PTY Master (Unix)
    -> Pseudo Console / PTY pair
      -> TUI Process
```

## 12.2 Windows-specific pitfalls (dedicated test matrix required)

*   ANSI support differences (legacy conhost vs Windows Terminal)
*   Unicode / wide-char width (CJK, emoji, box-drawing)
*   ConPTY alternate-screen + scrollback quirks
*   `CTRL+C` handling (console control events vs SIGINT)
*   Console codepage / UTF-8 (`chcp 65001`, `PYTHONUTF8=1`)
*   Resize behavior + reflow differences
*   Color depth negotiation (16 vs 256 vs truecolor)
*   Working directory: ConPTY children do not inherit the parent CWD unless
    set explicitly (they land in the profile dir). `tui-lab-pty` defaults
    unset `cwd` to the current process directory.
*   Child `stderr` shares the PTY stream by default (portable-pty inherits
    stdio); a separate stderr pipe for clean failure bundles is v2 work.
*   Kill semantics differ: `portable-pty` 0.9 `kill()` sends SIGHUP first on
    Unix (escalating to SIGKILL), versus TerminateProcess on Windows. A
    kill-path `close()` therefore reports signal `Hangup`, not `SIGKILL` —
    tests must accept that shape (seen in Ubuntu CI, Phase 6).
*   Key press AND release records: ConPTY delivers both, so apps (and the
    Phase 1 fixture) must filter on press events or every key acts twice —
    caught by the `runtime_smoke` test in Phase 1.
*   Mouse input, three ConPTY rules (proven live in Phase 9b — each cost a
    debugging cycle, recorded here so nobody re-pays them):
    1.  One write per input event. Glued escape sequences
        (`press+release` in a single write) are rejected by streaming
        parsers — the app died on the spot. Gestures always travel as
        distinct steps, exactly like real event streams.
    2.  Button-coded release only. ConPTY does not translate the generic
        SGR release (`ESC[<3;x;ym`) into a console input record — the
        event never arrives (app alive, pump clean, zero bytes). Release
        as `ESC[<<button>;x;ym` (e.g. `ESC[<0;30;2m` for left) works on
        ConPTY and parses on raw PTYs, so `RELEASE x y` means left.
    3.  No duplicate presses without release. Crossterm's Windows backend
        edge-detects on button state, so a second press while held produces
        no event — again exactly like physical hardware.

## 12.3 Mitigations

## 12.4 Phase 0 spike findings (Windows, PASS)

Throwaway crate `tuilab/spike-pty` (portable-pty 0.9 + alacritty_terminal 0.26,
`cmd /Q` child) verified on Windows 10.0.26200: spawn, prompt detect, echo
round-trip, grid shows output, PTY+grid resize 120x24 → 80x24, clean exit.

**Critical finding — answer the DSR handshake:** ConPTY opens with
`ESC [ 6 n` (cursor-position request). `alacritty_terminal` surfaces the reply as
`Event::PtyWrite`; the engine MUST forward it to the PTY master writer or the
session stalls (no prompt, no echo). Consequences for Phase 1:

*   `tui-lab-terminal` must pump every received chunk through the grid
    **immediately** (not in batch after the wait), so handshake responses go back
    without delay.
*   The `EventListener` needs shared (`Arc<Mutex<…>>`) access to the PTY writer.
*   `tui-lab-pty` API must expose `try_clone_reader` + `take_writer` + `resize`
    plus a bounded exit helper (`try_wait` poll → `kill` on timeout); a bare
    blocking `wait()` can hang the runner.
*   Pinned versions: `portable-pty = "0.9"`, `alacritty_terminal = "0.26"`,
    `vte = "0.15"` (feed via `vte::ansi::Processor::advance`, grid via
    `Term::grid()` / `renderable` reads, dims via `term::test::TermSize`).

*   Default `TERM=xterm-256color`, allow per-test override.
*   Resize test matrix: `80x24, 120x40, 160x50, 40x15` on both Win + Linux CI.
*   Separate `stderr` pipe so app logs don't pollute screen asserts.
*   Ship Windows CI job using Windows Terminal + conhost; fail loudly on ConPTY version < Win10 1809.
*   Packaging via `cargo-dist`; auto-update deferred to v3.

## 12.5 Linux input-timing rules (proven in CI, Phase 6)

Three CI failures (Ubuntu `proto` close, both-legs `search-flow`) traced to
one class: **input bytes lost to async races on Linux**. The engine, the
fixture, and the protocol were all correct — the *sequences* were not. Rules,
now enforced in our own suites and recommended to every user:

1.  **Never send input immediately after `resize`.** The redraw is async; a
    byte sent mid-redraw can vanish (observed: quit byte lost → SIGHUP kill
    path). Always follow `resize` with `wait_for_text` first (`docs/04` already
    prescribes this — now with CI evidence attached).
2.  **Never send a key immediately after a lone ESC.** Crossterm-class
    runtimes resolve a bare ESC with a short timeout, so `ESC` + fast `q`
    coalesces into Alt+`q` (ignored). Re-sync with `wait_for_text` between
    them. Sync via waits, never sleeps (`AGENTS.md` §3).
3.  **Prefer quit-in-close over press-quit-then-close.** `registry.remove`
    and both `close` actions accept quit input and poll for natural exit
    within grace before killing — one atomic call instead of two racy
    round-trips. Added in Phase 6 after the Ubuntu SIGHUP investigation.
