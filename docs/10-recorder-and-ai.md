# 10 — Recorder & AI Features

## 10.1 Recorder (`tuilab record`) — Playwright Codegen equivalent

```bash
tuilab record --command "./codegraph"
```

User interacts normally (Down, Enter, type search, Enter, q). Recorder captures key presses + screen transitions and emits YAML:

```yaml
steps:
  - wait_for_text: "Projects"
  - press: ENTER
  - press: "/"
  - type: "table"
  - press: ENTER
```

### Smart waits (required)

Generate `wait_for_text` between inputs, not `sleep: 2s`:

```yaml
# good
- wait_for_text: "Projects"
- press: ENTER
- wait_for_text: "Details"
# bad
- sleep: 2s
```

Heuristic: after each input, snapshot screen hash; when stable for N polls or target text appears, emit `wait_for_text` with the newly-appeared stable string.

## 10.2 AI test generation (v3)

*   **From README/source:** input `README.md` + app help text → output starter suites (`startup`, `search`, `navigation`, `quit`).
*   **From screen:** given grid, suggest assertions:

```
+---------------+
| Projects      |
| > Project 1   |
+---------------+
-> suggest: expect_text "Projects", expect_text "Project 1"
```

## 10.3 AI failure explanation

```
Expected: "Search Results"
Actual:   "No matches found" + screen [Search, table_]
-> "Search input was not focused before typing; missing '/' to enter search mode."
```

Feed the failure bundle from `11` (expected/actual/key history/dims) to the model.

## 10.4 Self-healing (opt-in, approval required)

If UI changes `Project Details` → `Details`, propose assertion update as a diff. Never auto-merge — it hides regressions. Gate behind `--self-heal=suggest` (default) vs `--self-heal=apply` (CI forbidden).
