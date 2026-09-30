# 18 - Application and Web Guide Audit (2026-09-30)

This is a black-box and documentation audit of the implementation currently in
the repository. It supplements `docs/17-analysis-findings.md`; it does not
claim that every item below is a runtime regression.

## 18.1 Validation performed

| Surface | Check | Result |
|---|---|---|
| Rust engine, CLI, MCP, SDKs | `cd tuilab; cargo test --workspace` | Pass: workspace tests green on Windows |
| Web guide | `cd web-guide; npm run build` | Pass; Vite reports a 651 kB minified JS chunk |
| Web guide behavior | `cd web-guide; npm run test:e2e` | Pass: 13 Chromium tests |
| CLI surface | `cargo run -p tui-lab-cli -- --help` and `run --help` | Commands and flags captured below |

The passing checks are useful evidence: the highest-value findings are contract
and coverage gaps that the current tests do not detect.

## 18.2 Findings

### P1 - The JSON Schema is not a schema for the implemented DSL

**Evidence:** `tuilab/schemas/test-schema-v1.json` defines detailed validation
only for `press` and `type`. `steps.items` has no `additionalProperties: false`
and no definitions for `wait_for_text`, `sleep`, `resize`, assertions,
`snapshot`, or `wait_for_exit`. `setup`, `cleanup`, and `assertions` are only
`{ "type": "array" }`. `environment` also accepts any value shape.

The Rust parser in `tuilab/crates/protocol/src/steps.rs` implements a much
larger strict DSL. This creates a false CI signal: the schema job can report
"valid" for a suite containing misspelled or structurally invalid steps, while
the Rust runner later rejects it or interprets a different shape.

**Cheap reproduction:** add `- wait_for_text: { typo: true }` or
`- made_up_step: true` to a suite and validate it with
`tuilab/schemas/test-schema-v1.json`. The JSON Schema accepts the step even
though the Rust parser rejects it.

**Action:** generate or hand-maintain a complete schema from the protocol types,
including all step aliases, scalar/map forms, assertions, tags, skip/focus,
attachments, budgets, and timing. Add negative schema tests and ensure the CI
validator uses the same strictness as `TestFile::from_yaml`.

### P1 - CLI help and README command inventory are stale

The actual top-level CLI exposes `init`, `run`, `report`, `record`, `proto`, and
`trace`. `debug` and `step` are not subcommands; they are `run` flags
(`--debug` and `--step`). `report` help still says "junit for now" even though
`report --format html` is implemented and covered by CLI tests.

The README component list describes `debug` and `step` as commands. This has a
direct usability cost because users commonly start from `tuilab --help` and
copy command names.

**Action:** align README component text, CLI doc comments, and guide examples
with the actual command tree. Add a test that checks documented command names
against `--help` output or maintain one generated command reference.

### P2 - The guide omits important application documents

The web guide has nine chapters, but the repository's design source of truth
also contains architecture, runtime, protocol, assertion internals, reporting,
cross-platform behavior, packaging, roadmap, and project scope documents. The
following topics are either absent as a dedicated page or only partially
represented:

* the three-layer architecture and ownership boundaries;
* the JSON-lines protocol, field/error contract, and versioning rules;
* PTY/ConPTY behavior, terminal emulation limits, and supported capabilities;
* report JSON/JUnit/HTML contracts and artifact layout for CI consumers;
* packaging/release status and the platform support matrix;
* security model details for MCP configuration, environment inheritance, and
  audit records;
* roadmap boundaries: what is intentionally not implemented (HTTP/WS,
  Studio, framework adapters, clipboard/scrollback, and remote agents).

These omissions make the site good as a quickstart but incomplete as the
application reference.

**Action:** add an "Architecture and protocol" reference chapter and a
"Configuration, reports, and releases" section inside the guide. Keep the
content self-contained and mark planned features explicitly so users can
distinguish a missing feature from an undocumented one.

### P2 - Frontend test coverage does not exercise responsive or accessible use

The 13 Playwright tests cover navigation, search, theme, copying, external URL
shape, and a basic vertical rhythm check. They run Chromium only and use the
default viewport. There is no mobile viewport pass, keyboard-only pass across
the full navigation, accessibility scan/assertion, reduced-motion check, or
verification that code examples use a currently valid command.

The build also emits a 651 kB minified JavaScript chunk. This is not a failure,
but it is a measurable performance risk for a documentation site and should be
tracked rather than hidden in build output.

**Action:** add desktop/mobile Playwright projects, keyboard focus assertions,
an accessibility check for each route, and a content smoke test for command
snippets. Split the main bundle or set and justify a documented budget.

### P2 - Documentation examples are not validated against the executable

The guide contains YAML, CLI, MCP, and three SDKs worth of snippets, but the
frontend tests only check that pages render and that one copied snippet
contains `pip install tui-lab`. They do not execute or parse representative
snippets, so drift can survive while all website tests remain green.

High-risk examples include installation commands before publication, the full
MCP configuration, YAML step shapes, SDK method signatures, and report/trace
commands.

**Action:** keep a small set of canonical snippets in executable fixtures or
extract snippets during CI and validate them with `tuilab --help`, YAML parsing,
SDK typechecks, and MCP tool discovery.

### P3 - Search is route-oriented rather than feature-complete

`web-guide/src/lib/search.ts` indexes one hand-written entry per route. It does
not index the source-of-truth docs, command help, or the actual MDX body. New
commands and terms can therefore be documented on a page but remain
undiscoverable unless someone manually updates the corpus.

**Action:** derive the search corpus from MDX content or generate it during the
build. Add search regressions for every top-level command and each MCP tool,
including `trace`, `proto`, and `--output-dir`.

## 18.3 Missing feature inventory

These are product gaps, not necessarily bugs in the current implementation:

| Area | Current state | User impact | Suggested next proof |
|---|---|---|---|
| Second framework fixture | Ratatui is the main end-to-end proof | Cross-framework claims remain lightly validated | Add a Textual, Bubble Tea, or shell fixture |
| Full resize matrix | A matrix flag exists, but coverage is fixture-focused | Narrow/wide layout regressions can escape | Add committed 40x15, 80x24, 120x40, and 160x50 coverage |
| Remote/API execution | HTTP/WS is future | No remote agents or hosted runners | Define authentication and a threat model first |
| Studio/test management | Planned v3 | No visual authoring or report-history UI | Keep the guide reference-only until the core API is stable |
| Framework semantics | Role tree is heuristic and black-box only | Complex locators remain text/coordinate based | Add opt-in adapters without weakening black-box testing |
| Terminal capability breadth | Sixel and basic mouse exist; broader capabilities remain limited | Some real TUIs cannot be tested faithfully | Publish a capability matrix and fixture each supported capability |

## 18.4 Recommended order

1. Replace the incomplete JSON Schema and add negative schema tests.
2. Fix release/install status and CLI command wording in README and guide.
3. Add protocol/architecture/configuration reference pages and link them from
   the guide.
4. Validate representative documentation snippets in CI.
5. Expand frontend checks to mobile, keyboard, accessibility, and bundle size.
6. Add a second-framework fixture and the four-size resize matrix before
   marketing broader compatibility.

## 18.5 Residual risk

The audit was run on Windows. The repository's CI targets Linux and macOS as
well, but this review did not independently execute those runners. Passing the
local workspace and Playwright suites establishes current Windows health, not
cross-platform parity.