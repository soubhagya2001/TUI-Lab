# 07 — CLI Reference (`tuilab`)

Universal entry point — works with any language app, no app changes.

## 7.1 Commands

```bash
# Scaffold project
tuilab init
# -> tuilab.yaml + tests/smoke.yaml

# Run all tests (defaults to the configured tests dir)
tuilab run

# Run one file / dir
tuilab run tests/search.yaml
tuilab run tests/ --terminal 120x40
# (--parallel arrives in v2; Phase 3 runs sequentially)

# Interactive recording (v2 — Phase 3 prints a pointer)
tuilab record --command "./codegraph"

# Re-render stored results as JUnit
tuilab report --format junit --out results.xml
# (--format html arrives in v2)

# Debug a failed run (full failure bundle: screens, history, diffs)
tuilab run tests/search.yaml --debug
# (--step arrives with the interactive runner in v2)
```

## 7.2 `tuilab.yaml` (project config)

```yaml
version: "1.0"
tests_dir: tests
snapshots_dir: tests/snapshots
default_terminal: { width: 120, height: 40 }
default_timeout: 10s
parallel: 4
report: { json: reports/results.json, html: reports/index.html, junit: reports/junit.xml }
env:
  TERM: xterm-256color
```

## 7.3 Exit codes

| Code | Meaning |
|------|---------|
| 0 | All passed |
| 1 | ≥1 test failed |
| 2 | Config/schema error |
| 3 | App launch / PTY error |
| 4 | Timeout / hung process killed |

## 7.4 CI example

```yaml
# .github/workflows/tui.yml
steps:
  - run: cargo build --release
  - run: tuilab run tests/
  - run: tuilab report --format junit --out results.xml
```

CI output:

```
TUI Lab Results
----------------
✓ startup
✓ navigation
✓ search
✗ invalid input
✓ graceful exit
4 passed, 1 failed
```

JUnit/JSON/HTML artifacts in `11-ci-reporting-debugging.md`.
