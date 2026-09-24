# 07 — CLI Reference (`tuilab`)

Universal entry point — works with any language app, no app changes.

## 7.1 Commands

```bash
# Scaffold project
tuilab init
# -> tuilab.yaml + tests/smoke.yaml

# Run all tests (defaults to the configured tests dir)
tuilab run

# Run one file / dir (sequential by default; --parallel fans out, run-all)
tuilab run tests/search.yaml
tuilab run tests/ --terminal 120x40
tuilab run tests/ --parallel 4
# Slots clamp to [1, registry cap 8]; results stay in input order; a failing
# suite never aborts its siblings.

# Select and harden runs: tags, shards, retries (see docs/05 §5.4)
tuilab run tests/ --tags smoke
tuilab run tests/ --shard 1/3
tuilab run tests/ --retries 2

# Resize matrix: every suite once per geometry (names suffixed @WxH)
tuilab run tests/ --resize-matrix 80x24,120x40

# Interactive recording (drives the app, emits YAML with smart waits)
tuilab record --command "./codegraph" --out tests/codegen.yaml
# --target python|js|rust emits SDK code; clicks synthesize CLICK steps
tuilab record --command "./codegraph" --target python --out tests/codegen.py
# end with Ctrl+\ or stdin EOF; quit the app first (see docs/10)

# Re-render stored results (JUnit or self-contained HTML)
tuilab report --format junit --out results.xml
tuilab report --format html --out index.html

# Debug a failed run (full failure bundle: screens, history, diffs)
tuilab run tests/search.yaml --debug

# Trace viewer: failures retain reports/traces/<suite>.zip (timeline +
# raw bytes); render it, replay output, or replay recorded inputs.
# --trace always|never overrides the retain-on-failure default.
tuilab trace reports/traces/search-flow.zip
tuilab trace reports/traces/search-flow.zip --replay
tuilab trace reports/traces/search-flow.zip --replay-input

# Extra report inputs: suite `attachments:` files copy to
# reports/attachments/<suite>/; every run appends reports/history.jsonl
# (flake tracking surfaced in HTML reports).

# Isolate this run's artifacts (results/junit/html/history/traces/
# attachments) under another directory — concurrent runs sharing a CWD
# (e.g. SDK `Runner.run` in parallel) then never clobber each other.
tuilab run tests/ --output-dir reports/run-42

# Step through a run interactively (Enter continues, q aborts; forces sequential)
tuilab run tests/search.yaml --step

# JSON-lines engine mode for SDK sidecars (docs/09) — one action in, one reply out
tuilab proto
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
