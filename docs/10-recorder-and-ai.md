# 10 — Recorder & AI Features

## 10.1 Recorder (`tuilab record`) — Playwright Codegen equivalent

```bash
tuilab record --command "./codegraph" --out tests/codegen.yaml
# end with Ctrl+\ (not forwarded) or stdin EOF; quit the app first (q)
```

User interacts normally (Down, Enter, type search, Enter, q) while the
recorder owns the PTY: stdin bytes go to the app, the app screen renders on
stdout, every input is logged. Beats are paced (8-byte chunks behind a
stabilize gate) so waits synthesize faithfully; original bytes are forwarded
verbatim so replay matches the session byte-for-byte:

```yaml
steps:
  - wait_for_text:
      text: "Projects"
  - press: ENTER
  - press: "/"
  - type: "table"
  - press: ENTER
```

### Smart waits (as built)

No sleeps are ever emitted. After each beat the screen must settle (3
identical polls at 50ms, 5s cap) before the next beat is consumed; the first
new stable line — box-drawing borders and list markers stripped, so
`│> alpha-table │` becomes the assertable `alpha-table` — becomes a
`wait_for_text`. Typing runs coalesce into single `type` steps; adjacent
splits from beat chunking merge at write time. Boot waits up to 10s for
first content; EOF drains up to 5s so piped scripts (which land before boot)
still settle. Limits, all documented in-code: spinner/clock screens never
stabilize (timeout, last text wins); Alt chords must arrive together;
a trailing lone ESC becomes an explicit press.

### Methodology cross-check (playwright-testing skill)

Mirrors Playwright codegen one-to-one: auto-waiting assertions instead of
hardcoded waits (their `waitForTimeout` ban is our no-`sleep` rule),
user-visible text targets instead of implementation details, and trace-on-
failure via the `docs/11` bundle (`--debug`). The skill's locator strategy
(role/text over CSS/XPath) is the terminal analogue of our cleaned wait
targets over raw grid cells.

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
