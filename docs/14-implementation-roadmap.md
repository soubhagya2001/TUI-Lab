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
TUI-Lab/                        # repo root
  docs/                         # design source of truth (this file is 14/14)
  tuilab/                       # Rust workspace (members = crates/*)
    crates/
      core/          # TestContext, runner, SessionRegistry, results
      protocol/      # JSON actions, YAML steps, schema, versioning
      runtime/       # wait_for_text, supervisor (timeouts/retries)
      pty/           # portable-pty wrapper (Unix + ConPTY), PtySession
      terminal/      # alacritty_terminal adapter, Emulator, DSR forwarding
      input/         # keyboard encoding (mouse deferred to v2)
      assertions/    # text/screen/process taxonomy + condition_from_json
      snapshots/     # golden store + diff + regex masking
      reporter/      # JSON / JUnit re-render (HTML deferred to v2)
      cli/           # `tuilab` (clap): init/run/report/record-stub/proto
      mcp/           # `tuilab-mcp` (rmcp stdio, 9 tools)
    schemas/
      test-schema-v1.json
    sdks/
      python/        # `tui-lab` pip package (sidecar over `tuilab proto`)
    tests/e2e/<suite>.yaml      # self-hosted dogfood suites
    tests/fixtures/ratatui-sample/  # minimal TUI fixture (menu+search+quit)
    tests/snapshots/            # committed goldens
    tuilab.example.yaml         # documented starting config
  .github/workflows/ci.yml      # rust / e2e / sdk-python jobs × Win+Linux
```

Still future: `sdks/{javascript,rust}/`, `examples/{python-textual,go-bubbletea,bash-tui}/`.

## 14.3 Phased plan (Phases 0–5 done, Windows-verified; see §14.6)

*   **Phase 0 — Scaffold + spike:** 11-crate workspace, dual-OS CI skeleton, PTY spike PASS (deleted).
*   **Phase 1 — Universal Runtime:** PTY launch, grid emulator, keyboard, screen capture, resize, exit handling + Ratatui fixture + `runtime_smoke`.
*   **Phase 2 — Test Protocol:** Frozen 9 actions, YAML `tui-lab/v1` steps, tokio waits/supervisor, assertion taxonomy, snapshot goldens + `single_session` proof.
*   **Phase 3 — CLI:** `tuilab init/run/report` (+ `record` stub), JSON + JUnit reports, failure bundle core, `tests/e2e` dogfood.
*   **Phase 4 — MCP:** `tui-lab-core` session registry + `tuilab-mcp` on `rmcp` 3.x with 9 `tui_*` tools (Modes A+B), allowlist + cwd jail.
*   **Phase 5 — SDKs (Python slice):** `tuilab proto` JSON-lines mode + wire-contract test, shared JSON→Condition mapper, Python `tui-lab` SDK + pytest suite, CI `sdk-python` job.
*   **Phase 6 — Verification closure:** pushed, dual-OS CI green, Ubuntu legs triaged (input-race class recorded in `12` §12.5).
*   **Phase 7 — v2a Recorder:** `tuilab record` with smart `wait_for_text` synthesis (never `sleep`); recorded YAML replayed green; methodology vs `playwright-testing` skill.
*   **Phase 8 — v2b Parallel fan-out:** `JoinSet` runner + `--parallel N` within the registry cap of 8; per-suite isolation; combined reports; deterministic ordering.
*   **Phase 9 — v2c in slices:** 9a HTML reports + `--step` runner (done) → 9b mouse input, full loop → 9c `region:` masks, separate `stderr`, numeric exit codes, per-cell `styled` screens.
*   **Phase 10 — SDK fast-follows:** JS/TS (`@tui-lab/sdk`) then Rust (`tui-lab-sdk`), same sidecar contract, mirrored fixture flows + CI jobs; no native bindings.
*   **Phase 11 — parked post-release (grill decision).** Optional element/focus-tree adapters via sidecar-file tree + extended asserts. Black-box path always stays.
*   **Phase 12 — Release hardening:** `cargo-dist` packaging, stale scaffold comments swept, dead `NotImplemented` variants evaluated, versioning/release process decided (§14.7). v3 (studio, AI, remote agents) stays tracked in §14.4, not planned in detail.

## 14.4 MVP scope

**MVP v1 (useful alone, DECIDED first slice):** minimal Ratatui sample app
(menu + search + quit) tested on Windows + Linux CI first. Then: PTY launch, keyboard,
text asserts, `wait_for_text`, snapshots, YAML, CLI runner, JSON report, exit code,
grid-crate-backed ANSI handling. `tuilab run test.yaml`.

**v2 (split into shippable slices — Phases 7–9):**
v2a recorder → v2b parallel fan-out → v2c HTML reports, mouse, `--step`,
`region:` masks, separate `stderr` pipe, numeric exit codes, per-cell screens.
Already landed ahead of schedule: snapshot diff, region asserts (engine),
CI integration, resize handling (full 4-size matrix still open).

**v3:** web dashboard/studio (Tauri+React), test management, AI generation/failure analysis, remote agents, cross-platform matrix, plugins, visual terminal diff.

### 14.4.1 TUI Lab Studio sketch (v3, from source thread §23)

Three panes — Tests | Terminal Preview | Assertions — with
`[Run] [Record] [Debug] [Generate Test]` actions, backed by the same core
engine (Tauri + React frontend, `xterm.js`-style rendering, HTML/SQLite
reports). CLI-first until the engine is stable; the studio is a viewer over
it, never a second implementation.

## 14.5 Acceptance criteria (updated as phases land)

*   [x] Workspace per `14.2` builds on Windows (`cargo fmt/clippy/test` green)
*   [x] Workspace builds + tests green on Linux (dual-OS CI green on `main`)
*   [x] `tuilab run` executes YAML suites against the sample Ratatui app (dogfood `tests/e2e` green)
*   [ ] Same coverage against a second framework app (Textual placeholder in `05` still open)
*   [x] `tui_launch/press/screen/wait/assert/close` work via MCP stdio (Mode A/B tests + handshake probe green on Windows)
*   [x] Snapshot round-trip + regex-mask tests pass; golden diff on mismatch
*   [ ] Resize matrix 4 sizes green (only 120x40 + 80x24 exercised so far)
*   [x] JUnit output renders from stored JSON (consumed by GitHub Actions; failure bundle prints with `--debug` in e2e job)

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
*   **Phase 4 — done (Windows-verified).** `tui-lab-core` session registry
    (cap 8, idle reaper, shared pump), `tuilab-mcp` on `rmcp` 3.x with the 9
    `tui_*` tools, allowlist + cwd jail enforced, `sensitive` redaction,
    `tuilab.example.yaml`, stdio handshake probe + Mode A/B tests green.
    Fixed en route: registry warm-up must feed (not discard) the ConPTY
    handshake; snapshot golden paths unified via `text_golden_path`.
*   **Phase 5 — done (Windows-verified).** `tuilab proto` JSON-lines mode
    (all 9 actions, contract test), shared JSON→Condition mapper (MCP
    deduplicated onto it), Python `tui-lab` SDK (`TuiTest` async API +
    `Runner.run`, 5 pytest green), `docs/09` fast-follow checklists for
    JS/TS + Rust, CI `sdk-python` job.
*   **Phase 6 — done (dual-OS green).** Pushed to `origin/main`; fixed the
    fresh-runner fixture-deps failure (`--offline` dropped), the e2e
    `--no-run` syntax error, and three Linux input-race failures (resize and
    lone-ESC sequencing, fixed with `wait_for_text` sync + quit-in-close).
    Root-cause class recorded in `12` §12.5. Remaining open boxes (second
    framework app, 4-size matrix) move with v2/Phase 10 work.
*   **Phase 7 — done (Windows-verified).** `tui-lab-input` key decoder
    (arrows/F-keys/CTRL/ALT/UTF-8 + decode→encode round-trip tests),
    `tuilab record` (PTY-owned capture, paced beats, smart `wait_for_text`
    synthesis with border-stripped targets, verbatim byte forwarding,
    Ctrl+\ stop chord), `record_replay` proof (scripted stdin → YAML with
    waits, no sleeps → replay green). Fixed en route: serde `!Variant`
    tags don't round-trip — `Step`/`SuiteAssertion` now serialize to the
    single-key-map shape they parse.
*   **Phase 8 — done (Windows-verified).** `tui-lab-core` `run_suites`
    (`JoinSet` + semaphore capped at 8, input-ordered results, run-all
    semantics with infra errors as failed results), `tuilab run --parallel`
    (default from `tuilab.yaml`, sequential path untouched), overlap + order
    + containment tests, `--parallel 2` added to the CI e2e job.
*   **Phase 9a — done (Windows-verified).** Self-contained HTML reports
    (`report --format html`, offline-capable) and the `--step` runner
    (per-step hook in core, CLI pause with Enter/q, sequential forcing,
    headless-safe on closed stdin).
*   **Phase 9b — done (Windows-verified).** SGR mouse encoder (`CLICK`,
    `RELEASE`, scroll names through `press`, 1-based coords) + always-on
    fixture capture with status readout + live click/scroll/release loop
    green. Fixed en route: glued sequences kill streaming parsers (one
    write per event); ConPTY needs button-coded release (generic Cb=3
    never arrives) — both recorded in `12` §12.2.
*   **Phase 9c — done (Windows-verified).** `region:` masks (rect/top/
    bottom, blank-then-regex precedence), numeric exit codes end to end
    (`SuiteResult` + MCP/proto, additive with back-compat test), styled
    cells (`Emulator::cells` → goldens → `tui_screen styled`). Child
    `stderr` separation recorded as decided-against on portable-pty 0.9
    (no redirect API) rather than built fragile.
*   **Phase 10 — done (Windows-verified).** `@tui-lab/sdk` (typed TS,
    `node:test`, mirrored flow green) + `tui-lab-sdk` workspace member
    (same surface, `tokio` tests green); `proto_roundtrip` unmodified —
    the shared contract held across all three clients. CI `sdk-js` job.
*   **Phase 11 — parked post-release (grill decision).** Sidecar-file tree
    + extended asserts, when it happens — not before release.
*   **Phase 12 — done (Windows-verified).** Dead `NotImplemented`
    variants + stale scaffold comments swept (behavior-free, gates prove
    it); `cargo-dist` packaging (`tuilab` + `tuilab-mcp` × Win/Linux/macOS
    archives, release workflow on `v*` tags, release profile builds
    green); versioning decided below.

## 14.7 Release process (DECIDED Phase 12 — overturn via grill)

*   Stay on `0.1.0` until the first tagged release, then `0.x.y`
    SemVer discipline: breaking pre-1.0 changes allowed, recorded in the
    release notes. SDK versions (`tui-lab`, `@tui-lab/sdk`, `tui-lab-sdk`)
    move in lockstep with the engine.
*   Cut a release: green `main` → `git tag vX.Y.Z` → `release.yml`
    (cargo-dist) builds archives + checksums for Win/Linux/macOS and
    attaches them to the GitHub Release. Linux/macOS artifacts are
    CI-built, never claimed from local builds.
*   Explicitly NOT published (yet): crates.io, PyPI, npm, installers
    beyond archives, auto-update. Registry publishing waits on demand.
