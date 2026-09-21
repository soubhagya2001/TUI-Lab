# TUI Lab — Playwright for Terminal Applications

> Language-agnostic, black-box testing for terminal apps. Launch any TUI in a
> real terminal, drive it with keys and text, and assert what the screen shows —
> from YAML suites, Python/JS/Rust SDKs, or your AI assistant. No app changes required.

**Guide:** https://soubhagya2001.github.io/TUI-Lab/ ·
**Releases:** https://github.com/soubhagya2001/TUI-Lab/releases ·
**License:** MIT OR Apache-2.0

## Why TUI Lab

Terminal apps are hard to test: they paint grids of characters, react to raw
key bytes, and behave differently on Windows (ConPTY) vs Unix (PTY). TUI Lab
gives them the Playwright treatment — a real terminal runtime plus a small,
stable protocol on top:

*   **Test anything** — Ratatui, Textual, Bubble Tea, Ink, plain shell scripts.
    If it runs in a terminal, TUI Lab can drive it.
*   **No instrumentation** — black-box at the byte level. Ship the same binary
    you test.
*   **Sync via waits, not sleeps** — every step polls with timeout + retry, so
    suites are fast locally and stable on slow machines.
*   **Deterministic snapshots** — mask clocks, PIDs, and IDs out of goldens.
*   **Record, don't hand-write** — `tuilab record` turns a live session into a
    suite with smart waits synthesized for you.
*   **Agents welcome** — a Model Context Protocol server lets AI assistants
    explore your app and triage failures.

## Install

```bash
pip install tui-lab            # Python — tuilab + tuilab-mcp land on PATH
uv add tui-lab                 # same wheel, uv project
npx @tui-lab/cli run tests     # Node — no install needed
cargo install tui-lab-cli tui-lab-mcp  # Rust — from crates.io
```

| Method | Windows x64 | Linux x64 | macOS arm64 | Notes |
|--------|-------------|-----------|-------------|-------|
| `pip` / `uv` (`tui-lab`) | ✓ | ✓ | ✓ | Platform wheel embeds both binaries |
| `npx` / `npm` (`@tui-lab/cli`) | ✓ | ✓ | ✓ | Per-platform optional packages |
| `cargo install` | ✓ | ✓ | ✓ | Compiles from source |
| GitHub Release archives | ✓ | ✓ | ✓ | Manual download, unzip, PATH |

## 60-second quickstart

```bash
tuilab init          # scaffolds tuilab.yaml + tests/smoke.yaml
tuilab run tests     # drives the sample app

# ✓ smoke
# 2 passed, 0 failed
```

Then point a suite at your own app:

```yaml
schema: tui-lab/v1
name: Navigation Test
application:
  command: "./myapp"
terminal:
  width: 120
  height: 40
steps:
  - wait_for_text: "Main Menu"
  - press: ENTER
  - wait_for_text: "Dashboard"
  - assert_text:
      contains: "Dashboard"
cleanup:
  - press: q
```

Full walkthroughs live in the [developer guide](https://soubhagya2001.github.io/TUI-Lab/).

## Components

*   **`tuilab`** — CLI: `init`, `run` (sequential + `--parallel`), `record`,
    `report` (HTML/JUnit), `debug`, `step`, `proto` (JSON-lines engine mode).
    Configured via `tuilab.yaml`.
*   **`tuilab-mcp`** — MCP server: nine `tui_*` tools for AI assistants, with
    allowlist + project-folder jail, secret redaction, 8-session cap.
*   **SDKs** — thin sidecars over `tuilab proto`: `tui-lab` (Python),
    `@tui-lab/sdk` (TypeScript), `tui-lab-sdk` (Rust).
*   **`tui-lab-core`** — the single Rust engine shared by CLI and MCP.

## Contact us

*   Email: [soubhagyaprusty36@gmail.com](mailto:soubhagyaprusty36@gmail.com)
*   LinkedIn: [soubhagya-prusty](https://linkedin.com/in/soubhagya-prusty-5424811b6)
*   GitHub: [soubhagya2001](https://github.com/soubhagya2001)

Bug reports: please include your suite file, the exit code, and
`reports/results.json`.

## For contributors

Design source of truth: `docs/` (vision, architecture, protocol, roadmap).
Agent conventions: [AGENTS.md](./AGENTS.md). Registry deploys:
[`docs/16-packaging-distribution.md`](./docs/16-packaging-distribution.md).
