# AGENTS.md — TUI Lab

> Instructions for AI coding agents working in this repo. Humans: see `README.md`.

## 1. What this is

**TUI Lab** — language-agnostic, black-box testing platform for terminal applications
("Playwright for Terminal Applications"). Rust core engine + `tuilab` CLI + `tuilab-mcp`
MCP server + thin SDKs. Design source of truth: `docs/` (14 files). Status: Phase 6
complete (dual-OS green), implementation per `docs/14-implementation-roadmap.md` Phases 7–12.

## 2. Read before coding

1. `docs/README.md` — doc map + 6 design principles
2. `docs/02-system-architecture.md` — 3-layer model (Integration → Protocol → Runtime)
3. `docs/04-test-protocol-spec.md` — the v1 protocol; all frontends speak it, none reimplement it
4. `docs/14-implementation-roadmap.md` — Rust stack, repo layout, MVP scope
5. The doc for whatever layer you touch (`03` runtime, `06` assertions, `08` MCP, …)

## 3. Frozen decisions (do not relitigate without asking)

* **Core language: Rust** (stable ≥ 1.75). Targets: `x86_64-pc-windows-msvc`,
  `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin` minimum.
* **Terminal grid: reuse `alacritty_terminal`** (or equivalent grid crate). Custom code is
  limited to PTY management, input encoding, protocol/assertions. No hand-rolled grid.
* **SDKs (Python/JS/Rust): thin sidecar clients** over stdio/JSON to the `tuilab` binary
  in v1. No native PTY code in SDKs. Native bindings deferred to v3+.
* **MCP security: allowlist + cwd jail.** `tui_launch` only runs commands matching
  `security.allow_commands` in `tuilab.yaml`; `cwd` must resolve under project root.
  `tui_type { sensitive: true }` redacts from logs. Max 8 concurrent sessions, 60s idle kill.
* **MVP slice first:** minimal Ratatui sample (menu + search + quit) green on Windows + Linux
  CI before any new feature work.
* **Sync via `wait_for_text`, never `sleep`.** Every wait/assert has timeout + poll + retry.

## 4. Available local skills (`.agents/skills/`)

| Skill | Use when |
|-------|----------|
| `rust-mcp-server-generator` | Phase 4 MCP server work |
| `github-actions-efficiency` | CI workflow changes (dual-OS matrix) |
| `playwright-testing` | Recorder/replay/assert methodology questions |
| `domain-cli` | `tuilab` CLI patterns |
| `tui-design` | Sample apps, TUI pattern questions |
| `grill-me` | Sharpen a plan before implementing it |

## 5. Repo layout (target, see `docs/14-implementation-roadmap.md` §14.2)

```
tuilab/
  crates/{core,protocol,runtime,pty,terminal,input,assertions,snapshots,reporter,cli,mcp}/
  schemas/test-schema-v1.json
  tests/e2e/<suite>.yaml        # TUI Lab self-tests (YAML suites, NOT inline scripts)
  tests/fixtures/               # sample TUIs (ratatui first, then textual/bubbletea/bash)
  examples/                     # user-facing examples per framework
  docs/
```

## 6. Coding conventions (mandatory)

### 6.1 Tests live in separate files — always

* Integration/e2e tests go in `tests/` (Rust) or `tests/e2e/*.yaml` (self-hosted suites).
  **Never embed test scripts inside `src/` logic files.**
* Small unit tests for pure functions may use `#[cfg(test)] mod tests` at the bottom of
  the file they test — nothing else. Anything touching PTY/process/time → `tests/`.
* Name test files after the behavior: `tests/resize_matrix.rs`, `tests/snapshot_masking.rs`,
  `tests/e2e/search_flow.yaml`. One behavior per file.

### 6.2 Reuse over duplication

* Shared logic belongs in the lowest sensible crate (`core`, `protocol`, `terminal`).
  CLI, MCP, and SDKs are **thin adapters** — if you copy-paste a block twice, extract it.
* New cross-cutting helper → put it in the owning crate's `utils`, never in `cli/` or `mcp/`.

### 6.3 `utils` and `constants` in every crate

Each crate MUST follow this shape as it grows:

```
src/
  lib.rs | main.rs
  constants.rs      # magic strings/numbers, key tables, defaults, timeouts
  utils.rs          # (or utils/ module) shared helpers for THIS crate only
  error.rs          # thiserror types; no anyhow in libraries (binaries may use anyhow)
```

* No hardcoded terminal sizes, key sequences, timeouts, or protocol strings inline —
  they go in `constants.rs` and are imported.
* `utils.rs` holds pure, unit-testable helpers. I/O and PTY handles stay in dedicated modules.

### 6.4 Rust hygiene

* `cargo fmt --check` and `cargo clippy -- -D warnings` must pass. No exceptions.
* `serde` for all protocol/YAML/JSON types; unknown fields deny in v1 strict mode,
  warn-and-ignore only where the spec says so.
* `tracing`, not `println!`, in libraries. Structured fields on session/step boundaries.
* Error handling: libraries return `Result<_, CrateError>`; attach step/session context at
  the orchestration layer so failures feed the bundle in `docs/11-ci-reporting-debugging.md`.

### 6.5 Docs discipline

* Behavior change → update the matching `docs/*.md` in the same change.
* New `DECIDED:` markers go through the user (grill first, then record).
* Mermaid/ASCII diagrams must still render after edits.

## 7. Verification before declaring done

```
cargo fmt --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
cargo test -p tui-lab-cli --test e2e          # once e2e harness exists
tuilab run tests/e2e/smoke.yaml               # self-hosted dogfood
```

Windows + Linux must both pass before merge (ConPTY parity is a top risk —
see `docs/12-cross-platform-strategy.md`).

## 8. Git norms

* Small, focused commits; conventional messages (`feat(pty): …`, `fix(emulator): …`,
  `docs(mcp): …`). Never commit `reports/`, `target/`, or snapshot `.new` files.
* No secrets in YAML fixtures; use `sensitive: true` typing paths in tests.
