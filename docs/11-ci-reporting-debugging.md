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

## 11.2 Report formats (v1: JSON + JUnit; v2: +HTML)

*   `reports/results.json` — full step trace, timings, screens.
*   `reports/junit.xml` — CI-native pass/fail.
*   `reports/index.html` (v2) — step timeline with screen thumbnails, diff view.

## 11.3 Failure bundle (capture on every failure)

Phase 3 implements the core subset; `stderr` tail separation and env
redaction beyond `sensitive` typing arrive in v2 (child stderr currently
shares the PTY stream — see `12` §12.2).

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
