# TUI Lab — Documentation Index

> **Mission:** TUI Lab is a language-agnostic, black-box testing platform for terminal applications. It provides a universal protocol, virtual terminal runtime, CLI, MCP server, and SDKs so developers and AI agents can launch, interact with, inspect, and validate any terminal application.

Distilled from the `TUI Testing System Design` ChatGPT thread (17 Sep 2026) and
fully audited against it — every section is captured in `docs/01`–`docs/14`.

## Product name

*   **Application:** TUI Lab
*   **CLI binary:** `tuilab`
*   **Config file:** `tuilab.yaml`
*   **MCP server binary:** `tuilab-mcp`
*   **Core library (Rust):** `tui-lab-core`

## Doc map (`docs/`)

| # | File | Contents |
|---|------|----------|
| 1 | [01-vision-scope.md](./docs/01-vision-scope.md) | Problem, users, black-box principle, app types supported |
| 2 | [02-system-architecture.md](./docs/02-system-architecture.md) | 3-layer model, component diagram, data flow |
| 3 | [03-terminal-runtime.md](./docs/03-terminal-runtime.md) | PTY/ConPTY, terminal emulator, screen buffer, input |
| 4 | [04-test-protocol-spec.md](./docs/04-test-protocol-spec.md) | Session-based JSON protocol v1 |
| 5 | [05-test-definition-dsl.md](./docs/05-test-definition-dsl.md) | Portable YAML schema `tui-lab/v1`, SDK examples |
| 6 | [06-assertion-snapshot-engine.md](./docs/06-assertion-snapshot-engine.md) | Assertion taxonomy, snapshots, determinism/masking |
| 7 | [07-cli-reference.md](./docs/07-cli-reference.md) | `init/run/record/report`, flags, examples |
| 8 | [08-mcp-server-spec.md](./docs/08-mcp-server-spec.md) | 9 MCP tools, Modes A/B, session handling |
| 9 | [09-sdk-integration-guide.md](./docs/09-sdk-integration-guide.md) | Thin-wrapper SDK strategy, Python/JS/Rust sketches |
| 10 | [10-recorder-and-ai.md](./docs/10-recorder-and-ai.md) | Recorder, smart waits, AI gen/explain/self-heal |
| 11 | [11-ci-reporting-debugging.md](./docs/11-ci-reporting-debugging.md) | CI flow, JUnit/HTML, failure bundle |
| 12 | [12-cross-platform-strategy.md](./docs/12-cross-platform-strategy.md) | Unix PTY vs Windows ConPTY, encoding/resize pitfalls |
| 13 | [13-feasibility-risks.md](./docs/13-feasibility-risks.md) | Feasibility table, 4 hard problems, mitigations |
| 14 | [14-implementation-roadmap.md](./docs/14-implementation-roadmap.md) | Language choice, repo layout, Phase 1-6, MVP v1/v2/v3 |
| 15 | [15-web-guide-plan.md](./docs/15-web-guide-plan.md) | `web-guide/` React docs site plan (stack, pages, deploy) |

## Design principles (read first)

1.  **Protocol-first, not SDK-first.** One protocol consumed by CLI, MCP, SDKs, future HTTP/WS.
2.  **Black-box by default.** Test at the terminal byte level. No app changes required.
3.  **Single core engine.** CLI and MCP share `tui-lab-core`. No duplicate logic.
4.  **Session-based.** Persistent PTY sessions identified by `session_id`.
5.  **Sync via waits, not sleeps.** Every assertion has timeout + retry + polling.
6.  **Deterministic snapshots via masking.** Time/CPU/RAM/IDs must be maskable.

## How to read these docs

*   New to the project → read `docs/01`, `docs/02`, `docs/14` in order.
*   Implementing runtime → read `docs/03`, `docs/04`, `docs/06`, `docs/12`.
*   Implementing integrations → read `docs/04`, `docs/07`, `docs/08`, `docs/09`.
*   Planning QA/CI → read `docs/05`, `docs/06`, `docs/11`.

## For coding agents

See [AGENTS.md](./AGENTS.md) — mandatory conventions (tests in separate files,
reuse over duplication, `utils`/`constants` per crate, verification gates).
