# 17 — Analysis Findings, Phased

> Brief audit of `tuilab/crates/*`, `tuilab/sdks/*`, `.github/*`, `web-guide/`. Status as of `0.1.0` pre-tag.
> Spot-verified 2026-09-21: `web-guide/search.ts` invents `tui_resize` (real tools per `docs/08` are launch/press/type/screen/wait_for_text/assert/snapshot/run_test/close — no resize, no `tui_wait`); `docs/README.md` missing; `12 §12.3` body empty; `web-guide/README.md` still Vite template; `AGENTS.md` still says 14 files / Phase 6 (now 17 files, Phase 12 done). `git ls-files` shows NO tracked `dist/*.tgz`, `uv.lock`, `reports/results.json`, or `target/*.pdb` — that risk is clear, keep the gitignore guards.

## 17.0 Phase map

Work is ordered by blast radius: security/correctness first, robustness second, SDK/packaging third, docs+CI/repo hygiene alongside, Playwright-parity features last (each grilled before build).

| Phase | Scope | Gate before next phase |
|-------|-------|------------------------|
| P0 — Security & correctness (§17.1) | S1–S5, C1–C5 | Each fix has a regression test; `fmt`/`clippy -D warnings`/`test --workspace` green Win+Linux |
| P1 — Robustness (§17.2) | R1–R11 | No `expect`/`unwrap` in libs, no swallowed errors, dead guards wired or deleted |
| P2 — SDK & packaging (§17.3) | K1–K5 | SDK suites green (pytest 8, npm 5, cargo); wheel + npm tarball pack verified; version-gate test in CI |
| P3 — Docs & repo hygiene (§17.4) | D1–D4 | `docs/` renders consistently; `git status` clean of artifacts; README install note honest |
| P4 — CI hardening (§17.5) | I1–I5 | Matrix green incl. new legs; failure artifacts upload; release dry-runs pass |
| P5 — Playwright parity (§17.6) | F1–F8, one grill per feature | Grill first; black-box path always stays; no feature without e2e proof |

Rule: phases run in order; P5 items may be reordered by user priority after grill. Small doc-only fixes (D-items) may land anytime.

## 17.1 Phase P0 — Security / correctness (fix first)

### MCP security

| # | Finding | Location |
|---|---------|----------|
| S1 | MCP holds global `Mutex` across full `wait_for_text` pumps — one slow wait serializes all 8 sessions. Use per-session locks. | `crates/mcp/src/handler.rs:52,240-253` |
| S2 | Config writes `"./*"` (glob) but MCP enforces regex `^\./.*` — over-block/bypass. Unify to regex. | `tuilab.example.yaml:15-21`, `crates/mcp/src/security.rs:22` |
| S3 | `tui_run_test` joins relative paths / allows absolute paths without `jail()` — path traversal. Jail parent dir. | `crates/mcp/src/handler.rs:337-341` |
| S4 | `launch.env` passes `LD_PRELOAD/PATH/CARGO_*` through. Blocklist sensitive vars. | `crates/mcp/src/handler.rs:109`, `crates/cli/src/proto.rs:85` |
| S5 | `last_screen` unescaped in JUnit/HTML — breaks XML, stored-XSS. Escape + strip `0x00-0x08`. | `crates/reporter/src/junit.rs:43-47`, `crates/reporter/src/html.rs:45-51` |

### Engine correctness

| # | Finding | Location |
|---|---------|----------|
| C1 | Non-zero exits mapped to `(None,crashed=true)` — `ExitCode(2)` never passes via MCP/proto. | `crates/mcp/src/handler.rs:423-438`, `crates/cli/src/proto.rs:201-205` |
| C2 | `assert_text` is single `pump()` + one-shot `evaluate()` — only `wait_for_text` retries. Add `assert --timeout` poll loop. | `crates/core/src/runner.rs:344-373` |
| C3 | `screen_changed=true` hardcoded; `ExactText/CursorPosition/ExitCode/NotCrashed` unreachable from YAML. | `crates/core/src/runner.rs:344-373` |
| C4 | Snapshot step writes `dir/name.txt` manually, ignores `{WxH}.json` convention — sizes collide. | `crates/core/src/runner.rs:439-446`, `crates/snapshots/src/store.rs:199-214` |
| C5 | `default_timeout/terminal/env` + suite `terminal.timeout` parsed but never wired/enforced; `run_with_timeout` has zero callers. | `crates/cli/src/commands.rs:189-193,236-240`, `crates/protocol/src/steps.rs:86` |

## 17.2 Phase P1 — Robustness

| # | Finding | Location |
|---|---------|----------|
| R1 | `Regex::new` every poll tick — compile once. | `crates/runtime/src/wait.rs:22-23` |
| R2 | `decode_csi` returns `None` for unknown seqs — recorder accumulates `carry` instead of skipping. | `crates/input/src/decode.rs:105,125` |
| R3 | Region assert counts chars not cells; OOB yields misleading error. | `crates/core/src/runner.rs:375-388` |
| R4 | Duration `"m"` unchecked `*60` overflow; bare `"500"`→ms undocumented. | `crates/protocol/src/steps.rs:281-295` |
| R5 | Exit codes via `message.starts_with(...)` — match `CoreError` enum. | `crates/cli/src/commands.rs:166-174` |
| R6 | Dead guards: `clamp_dims/backoff_ms/clamp_timeout_ms` never called; `resize` accepts any `u16`. | `crates/terminal/src/utils.rs:7`, `crates/runtime/src/utils.rs:8` |
| R7 | Swallowed errors (`let _`): DSR replies, `quit` bytes, recorder forward, `proto respond` — ConPTY stalls silently. | — |
| R8 | `init` can write empty yaml on serialize fail; `read_to_string().unwrap_or_default()` hides I/O errors. | — |
| R9 | Only `ext=="yaml"` (misses `.yml`); follows symlinked dirs, no cycle guard; `parallel` silently clamps. | — |
| R10 | `pump_once` refreshes `last_active` on every read — idle reaper never fires for pollers. | `crates/core/src/sessions.rs:205-210` |
| R11 | `expect`/`unwrap` in libs — use `ok_or_else` / `LazyLock`. | `steps.rs:334,370`, `parallel.rs:36`, `security.rs:23` |

Maps to old enhancements: wire config + suite timeout (C5); polling asserts + regex-once (C2/R1); escape screens (S5); `clamp_dims` in both `resize()` paths (R6); `thiserror` + `#[source]` and enum exit codes (R5).

Follow-up audit pass (2026-09-24) re-verified every row against the tree:
P0 (S1–S5, C1–C5), P1 (R1–R9), P2 (K1–K2), P3 (D1–D4), P4 (I1–I5) are
implemented with regression tests. Five items were not, and are now fixed:

* **R10** — `pump_once` no longer stamps `last_active`; reads are not usage.
  Client-driven paths (writes, resize, launch response) stamp explicitly via
  `get_mut`/the launch path, so in-flight waits still survive a concurrent
  reap. Tests: `core/tests/sessions.rs::pump_once_does_not_refresh_activity`,
  `::client_access_refreshes_activity`, `mcp/tests/mcp_tools.rs::idle_sessions_reaped_on_tool_entry`.
* **R11** — the last production `expect` (default allowlist `OnceLock`) now
  fails **closed** to a deny-all list instead of panicking the MCP server.
* **K3** — `tuilab run --output-dir DIR` rebases every artifact (results,
  junit, html, history, traces, attachments); all three SDKs expose it
  (`reports_dir=` / third arg / `Runner::run_with_reports_dir`) so parallel
  runs in one CWD stop clobbering each other. Tests: `cli_e2e.rs::output_dir_rebases_every_report_artifact`
  plus per-SDK isolation tests.
* **K4** — leak guards everywhere: `PtySession::drop` kills a never-closed
  child, and each SDK kills a forgotten sidecar (Rust `Drop`, Python
  `__del__`, JS `Symbol.dispose`/`Symbol.asyncDispose`). Tests:
  `sdks/rust/tests/leak_guard.rs`, `test_sdk_surface.py::test_dropped_connection_kills_the_sidecar`,
  JS `Symbol.dispose kills a forgotten sidecar`.
* **K5** — a PR-time `pack-gate` CI job runs `bump_versions.py --check`
  (all 7 sites) plus real wheel/npm pack dry-runs, so drift fails on the PR
  instead of at release; the JS SDK no longer collapses unknown arches to x64.

Still open (accepted, see the audit rows): D4's `.agents/skills/` gitignore
exception is intentionally not applied (skills stay untracked by decision).

## 17.3 Phase P2 — SDK / packaging

| # | Finding |
|---|---------|
| K1 | No `assert(condition)` in Python/JS/Rust SDKs; `timeout_ms/poll_ms/styled/signal` dropped; `close` can't send graceful `q`. |
| K2 | Rust `find_binary` order `explicit→ENV→PATH→workspace`, no `_bin`/platform lookup — reintroduces PATH-loop bug; `LaunchOptions::default` gives `0x0` PTY. |
| K3 | `Runner.run` hardcodes CWD-relative `reports/results.json`, no mtime/clean — parallel runs overwrite, stale files pass. |
| K4 | No `Drop`/`asyncDispose` guards — leaked sidecar+PTY; double-close → 60s hang. |
| K5 | 6 version sites (`Cargo`, `pyproject`, `package.json`, `npm-cli` + 3 platforms) with no CI gate despite lockstep rule; `pack_wheel` picks lexicographically-last wheel, no `chmod +x`; `resolve.js` maps any non-arm64→x64; committed `dist/*.tgz`. |

Note on K5: `git ls-files` confirms no `dist/*.tgz` is actually tracked — keep it that way (gitignore guard) and fix pack-script selection + `resolve.js` mapping. Add a CI version-gate test + version-bump script.

## 17.4 Phase P3 — Docs & repo hygiene

| # | Finding |
|---|---------|
| D1 | Update `04` (mouse/styled/mask/numeric exit); resolve ignore-vs-deny (`04 §4.4` vs AGENTS §6.4); mark stderr future. |
| D2 | Fix `README` install claims vs `16` implemented-not-published — until first tag, point users at `cargo run` (README Install section needs a pre-publish note). |
| D3 | Bump AGENTS.md status (Phase 6→12 done, 14→17 files); create missing `docs/README.md`; fill empty `12 §12.3`. |
| D4 | Web-guide: replace Vite-template `README`; search is 9-entry `CORPUS` (verified — also fix invented `tui_resize`/`tui_wait` entries in `search.ts:47`); add kbd nav/empty/highlight states; flat nav, `sources[]` never rendered; `*→<Home/>` swallows 404; `guide.spec.ts` live-GETs externals (flaky — mock or drop); drop unused shadcn `Table/Empty/Skeleton/Dialog`. |

Tracked-but-ignored watchlist (verified clean 2026-09-21 — re-check before each release): `reports/results.json`, `web-guide/dist/*`, `uv.lock`, `target/*.pdb`. `.gitignore` hides `.agents/` yet AGENTS advertises `.agents/skills/` — un-ignore via `!.agents/skills/`.

## 17.5 Phase P4 — CI hardening

| # | Finding |
|---|---------|
| I1 | No macOS leg (ships untested) and no Rust-SDK job. |
| I2 | No schema gate (`schemas/test-schema-v1.json` vs YAML suites). |
| I3 | No `cargo-dist plan` check on PRs. |
| I4 | No artifact upload on failure (Rust reports, Playwright traces). |
| I5 | No `paths:` filters / `concurrency` / `rust-cache` (see `github-actions-efficiency` skill). |

## 17.6 Phase P5 — Extra features (Playwright parity, grill each)

Locators/role + a11y-tree dump · trace viewer (`trace.zip`: timeline + raw bytes + replay) · `record --target=python|js|rust` + mouse synthesis · fixtures/sharding (`--shard`, `--retries`, `skip/focus`, tags) · pixel/Sixel diff · fake timers + fetch stubs · perf budgets (`startup/render/latency`) · scrollback search, clipboard, hover/drag, resize-matrix helper · report attachments + waterfall + flake history · MCP `tui_resize` (real tool, not the `web-guide/search.ts:47` invention) + audit logging.

Status (grilled program order A→E2): A fixtures ✓ · B1 trace ✓ · B2 reports ✓ · C1 record ✓ · C2 terminal ✓ · D1 pixel/Sixel ✓ · D2 budgets ✓ · **E1 a11y/role ✓** · **E2 timing ✓ (P5 complete)**.

Follow-up on E2: a per-step `type` `delay` is now the inter-character gap
only — the pre-write pause comes from `timing.input_delay` — and the runner
sleeps *between* characters instead of after the last one, matching the MCP,
proto, and SDK `type` paths. See `05` §5.3 and `03` §3.5.
