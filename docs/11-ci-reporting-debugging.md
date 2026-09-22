# 11 — CI, Reporting & Failure Debugging

## 11.1 CI architecture

```
GitHub Actions / GitLab / Jenkins
  -> tuilab run
    -> Test Runner (tokio tasks, parallel)
      -> PTY Sandbox per test
        -> TUI Application
    -> Results: JUnit XML + HTML + screenshots + terminal logs + exit code
```

Example assertion in CI: build release binary, then `tuilab run`.

Our own pipeline (`.github/workflows/ci.yml`, P4): `rust` (fmt/clippy/test)
and `e2e` dogfood run on Windows + Linux + macOS; `sdk-python`/`sdk-js` on
Windows + Linux; `sdk-rust` and the `schema` gate (every `tests/e2e/*.yaml`
validated against `schemas/test-schema-v1.json`) on Ubuntu. Rust jobs skip
web-guide-only pushes (`paths:`), share a `rust-cache`, and cancel stale
runs via `concurrency`. Every job uploads its `reports/` (results,
snapshots, failure bundles) as an artifact on failure — never just red logs.

Parallel runs (`--parallel N`) fan suites out over a semaphore (capped by
the registry limit of 8) with run-all semantics: every suite completes, no
sibling aborts, and `results.json` / JUnit reassemble in input order so
reports stay diffable run-over-run.

## 11.2 Report formats (v1: JSON + JUnit; v2: +HTML)

*   `reports/results.json` — full step trace, timings, screens.
*   `reports/junit.xml` — CI-native pass/fail.
*   `reports/index.html` — self-contained step timeline with screens and diffs (inline CSS, no external assets).

## 11.3 Failure bundle (capture on every failure)

Failing suites additionally retain `reports/traces/<suite>.zip`: a
dependency-free stored-zip holding `trace.json` (step timeline with
`started_ms`/`duration_ms`, failure evidence, chunk index) and `pty.bin`
(raw bytes, capped at 256 KiB). `tuilab trace <zip>` renders the timeline;
`tuilab trace <zip> --replay` streams the bytes with original pacing
(gaps capped at 1s). Use `--trace always|never` to override the
retain-on-failure default.

Phase 3 implements the core subset; env redaction beyond `sensitive
typing is future work. Child `stderr` shares the PTY stream: `portable-pty`
0.9 exposes no stderr redirect (verified in vendored source — stdio is
hardwired to the slave), and per-OS shell-wrapper redirection was rejected
as fragile, so the separate-pipe item stays open pending a PTY upgrade
(decided Phase 9c, not deferred by neglect).

1.  test step number + action
2.  expected screen/text
3.  actual screen (text grid)
4.  raw terminal output (last N KB)
5.  screenshot / cell-snapshot JSON
6.  key event history
7.  process stdout/stderr tail
8.  exit code / signal
9.  terminal dimensions + `$TERM`
10. env vars (redacted secrets)
11. execution duration + per-step timings

Example:

```
FAIL: search-project, step 5
Expected text: "Search Results"
Actual:
+---------------------------+
| Search                    |
| table_                    |
+---------------------------+
Last input: ENTER
Possible issue: search command did not execute.
```

This beats bare `Assertion failed` and powers AI explanation in `10`.
