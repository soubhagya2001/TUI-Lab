# 14 — Implementation Roadmap (TUI Lab)

## 14.1 Language choice: Rust

Decision per user request ("fast + all support available"): **Rust for the core engine, CLI, and MCP server.**

| Need | Rust answer |
|------|-------------|
| Speed (PTY pump, grid diff, parallel tests) | Native, zero-GC, `tokio` tasks |
| PTY cross-platform | `portable-pty` (Unix PTY + Win ConPTY) |
| Terminal grid | `alacritty_terminal` (DECIDED: reuse, not custom buffer) |
| ANSI parsing | via grid crate (`vte`-compatible) |
| CLI | `clap` |
| Async runtime | `tokio` |
| Serde YAML/JSON/regex | `serde` + `serde_yaml` + `serde_json` + `regex` |
| Logging/tracing | `tracing` |
| Reports | JUnit XML + HTML builders |
| MCP server | `rmcp` (Rust MCP SDK, stdio transport) |
| Packaging | `cargo-dist`, single static binary (easy CI install) |
| Future desktop | `Tauri` (Rust backend) + React + xterm.js |

Alternatives rejected: Python/Node (fast to prototype, weak for ConPTY parity + single-binary distribution + emulator perf); Go (good PTY story, weaker `vte`-equivalent + MCP ecosystem vs Rust here). SDKs in other languages come later as thin clients — core stays Rust.

MSRV: stable Rust ≥ 1.75. Targets: `x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin` minimum.

## 14.2 Repository layout

```
tuilab/
  crates/
    core/          # TestContext, orchestrator, lifecycle
    protocol/      # JSON protocol types, schema, versioning
    runtime/       # supervisor (timeouts/retries/parallel)
    pty/           # portable-pty wrapper (Unix + ConPTY)
    terminal/      # vte parser + grid buffer + snapshots
    input/         # keyboard/mouse encoding
    assertions/    # text/screen/process/interaction/perf
    snapshots/     # golden store + diff + masking
    reporter/      # JSON / JUnit / HTML
    cli/           # `tuilab` (clap): init/run/record/report
    mcp/           # `tuilab-mcp` (rmcp stdio, 9 tools)
  schemas/
    test-schema-v1.json
  docs/            # this folder (docs-only phase)
```

Future: `sdks/{python,javascript,rust}/`, `examples/{python-textual,rust-ratatui,go-bubbletea,bash-tui}/`.

## 14.3 Phased plan

*   **Phase 1 — Universal Runtime:** PTY launch, emulator, keyboard, screen capture, resize, exit handling. No MCP.
*   **Phase 2 — Test Protocol:** Freeze 9 actions (`launch/press/type/wait/screen/assert/snapshot/resize/close`). Stabilize.
*   **Phase 3 — CLI:** `tuilab run test.yaml` validates platform end-to-end.
*   **Phase 4 — MCP:** Same protocol as `tui_*` tools (Modes A+B).
*   **Phase 5 — SDKs:** Python → JS/TS → Rust thin wrappers.
*   **Phase 6 — Instrumentation:** Optional element/focus tree adapters (Ratatui/Bubble Tea/Textual). Never required.

## 14.4 MVP scope

**MVP v1 (useful alone, DECIDED first slice):** minimal Ratatui sample app
(menu + search + quit) tested on Windows + Linux CI first. Then: PTY launch, keyboard,
text asserts, `wait_for_text`, snapshots, YAML, CLI runner, JSON report, exit code,
grid-crate-backed ANSI handling. `tuilab run test.yaml`.

**v2:** resize matrix, mouse, region asserts, snapshot diff, HTML reports, recorder, CI integration, parallel execution.

**v3:** web dashboard/studio (Tauri+React), test management, AI generation/failure analysis, remote agents, cross-platform matrix, plugins, visual terminal diff.

### 14.4.1 TUI Lab Studio sketch (v3, from source thread §23)

Three panes — Tests | Terminal Preview | Assertions — with
`[Run] [Record] [Debug] [Generate Test]` actions, backed by the same core
engine (Tauri + React frontend, `xterm.js`-style rendering, HTML/SQLite
reports). CLI-first until the engine is stable; the studio is a viewer over
it, never a second implementation.

## 14.5 Acceptance criteria (docs phase → build phase)

*   [ ] `cargo new` workspace per `14.2` builds on Win + Linux
*   [ ] `tuilab run` executes `05` full example against a sample Ratatui + Textual app
*   [ ] `tui_launch/press/screen/wait/assert/close` work via MCP stdio against same apps
*   [ ] Snapshot round-trip + mask test passes; resize matrix 4 sizes green
*   [ ] JUnit output consumed by GitHub Actions with failure bundle from `11`

## 14.6 Phase status (updated as phases land)

*   **Phase 0 — done (Windows-verified).** Workspace scaffolds 11 crates;
    dual-OS CI workflow added (Linux leg runs on first push); PTY spike
    PASSED on Windows (findings in `12` §12.4) and was deleted.
*   **Phase 1 — done (Windows-verified).** `tui-lab-pty` (spawn/pump/resize/
    bounded close), `tui-lab-terminal` (grid adapter + DSR forwarding),
    `tui-lab-input` (xterm key table), minimal Ratatui fixture, and
    `runtime_smoke` (boot → select → resize → search → quit, exit 0) all green
    under `cargo fmt/clippy/test`. Caught + fixed: ConPTY press/release
    double-fire (`12` §12.2).
*   **Phase 2 — done (Windows-verified).** `tui-lab-protocol` (9 JSON actions,
    strict field validation; YAML `tui-lab/v1` steps with aliases), `tui-lab-runtime`
    (tokio `wait_for_text` + `run_with_timeout`), `tui-lab-assertions` (8-condition
    taxonomy over `ScreenView`), `tui-lab-snapshots` (goldens, regex masks, unified
    diff), and a `single_session` YAML end-to-end proof against the fixture.
    Single-session only; registry, `TestContext`, reporters deferred.
*   **Phase 3 — done (Windows-verified).** `tui-lab-core` runner
    (`TestContext`, step pipeline, cleanup-always, exit assertions),
    `tui-lab-reporter` (JSON + JUnit re-render), `tuilab` CLI
    (`init/run/report`, `record` stubbed for v2, sequential runs),
    self-hosted `tests/e2e` dogfood (smoke + search-flow with committed
    golden) green via `tuilab run tests/e2e`, e2e job added to CI.
    Fixed en route: ConPTY children need explicit CWD (`12` §12.2).
*   **Phase 4 — next.** MCP server (`rmcp`, 9 tools, allowlist + cwd jail).
