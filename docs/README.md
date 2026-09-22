# TUI Lab — Documentation Index

> **Mission:** TUI Lab is a language-agnostic, black-box testing platform for terminal applications. It provides a universal protocol, virtual terminal runtime, CLI, MCP server, and SDKs so developers and AI agents can launch, interact with, inspect, and validate any terminal application.

## Doc map

| # | File | Contents |
|---|------|----------|
| 1 | [01-vision-scope.md](./01-vision-scope.md) | Problem, users, black-box principle, app types supported |
| 2 | [02-system-architecture.md](./02-system-architecture.md) | 3-layer model, component diagram, data flow |
| 3 | [03-terminal-runtime.md](./03-terminal-runtime.md) | PTY/ConPTY, terminal emulator, screen buffer, input |
| 4 | [04-test-protocol-spec.md](./04-test-protocol-spec.md) | Session-based JSON protocol v1 |
| 5 | [05-test-definition-dsl.md](./05-test-definition-dsl.md) | Portable YAML schema `tui-lab/v1`, SDK examples |
| 6 | [06-assertion-snapshot-engine.md](./06-assertion-snapshot-engine.md) | Assertion taxonomy, snapshots, determinism/masking |
| 7 | [07-cli-reference.md](./07-cli-reference.md) | `init/run/record/report`, flags, examples |
| 8 | [08-mcp-server-spec.md](./08-mcp-server-spec.md) | 10 MCP tools, Modes A/B, session handling, audit log |
| 9 | [09-sdk-integration-guide.md](./09-sdk-integration-guide.md) | Thin-wrapper SDK strategy, Python/JS/Rust sketches |
| 10 | [10-recorder-and-ai.md](./10-recorder-and-ai.md) | Recorder, smart waits, AI gen/explain/self-heal |
| 11 | [11-ci-reporting-debugging.md](./11-ci-reporting-debugging.md) | CI flow, JUnit/HTML, failure bundle |
| 12 | [12-cross-platform-strategy.md](./12-cross-platform-strategy.md) | Unix PTY vs Windows ConPTY, encoding/resize pitfalls |
| 13 | [13-feasibility-risks.md](./13-feasibility-risks.md) | Feasibility table, 4 hard problems, mitigations |
| 14 | [14-implementation-roadmap.md](./14-implementation-roadmap.md) | Language choice, repo layout, phases, MVP v1/v2/v3 |
| 15 | [15-web-guide-plan.md](./15-web-guide-plan.md) | `web-guide/` React docs site plan (stack, pages, deploy) |
| 16 | [16-packaging-distribution.md](./16-packaging-distribution.md) | Registry deploys: PyPI/npm/crates.io per-platform steps |
| 17 | [17-analysis-findings.md](./17-analysis-findings.md) | Audit findings phased P0–P5 with gates |

## Design principles (read first)

1.  **Protocol-first, not SDK-first.** One protocol consumed by CLI, MCP, SDKs, future HTTP/WS.
2.  **Black-box by default.** Test at the terminal byte level. No app changes required.
3.  **Single core engine.** CLI and MCP share `tui-lab-core`. No duplicate logic.
4.  **Session-based.** Persistent PTY sessions identified by `session_id`.
5.  **Sync via waits, not sleeps.** Every assertion has timeout + retry + polling.
6.  **Deterministic snapshots via masking.** Time/CPU/RAM/IDs must be maskable.

## How to read these docs

*   New to the project → read `01`, `02`, `14` in order.
*   Implementing runtime → read `03`, `04`, `06`, `12`.
*   Implementing integrations → read `04`, `07`, `08`, `09`.
*   Planning QA/CI → read `05`, `06`, `11`.
*   Shipping releases → read `16`, then `17` for remaining work in phase order.
