# 16 — Packaging & Distribution (registries, per platform)

> **Status:** implemented, not yet published. Code, pack scripts, and the
> `publish.yml` workflow are in place and locally verified; the first
> `vX.Y.Z` tag + one-time registry setups (below) execute the real publish.

## 16.1 What users get

| Command | Registry | Contents |
|---------|----------|----------|
| `pip install tui-lab` / `uv add tui-lab` / `uvx tui-lab …` | PyPI (`tui-lab`) | Sidecar + embedded `tuilab`/`tuilab-mcp`; console scripts put both on PATH |
| `npx @tui-lab/cli …` / `npm i @tui-lab/sdk` | npm (`@tui-lab/cli`, `@tui-lab/sdk`) | Node launcher + per-platform binary packages |
| `cargo install tui-lab-cli tui-lab-mcp` | crates.io | Compiled from source |
| GitHub Release archives | (existing `release.yml`) | Manual download per OS |

Platform coverage is identical everywhere: Windows x64, Linux x64, macOS
arm64 — the three cargo-dist targets. Versions move in lockstep (`docs/14`
§14.7); `publish.yml` fails fast if any package version differs from the tag.

## 16.2 Anti-loop rule (why resolution order matters)

The `tuilab` entry points are *launchers*, not engines. Spawning a launcher
as the engine re-execs forever (proven by `test_launcher.py`). Hence:

*   Launchers resolve via `find_engine` (Python) / direct platform-package
    lookup (Node): `TUILAB_BIN` → bundled binary → workspace build.
    **PATH is never consulted.**
*   SDK `find_binary` prefers bundle/workspace over PATH for the same reason;
    PATH stays last as a manual-install fallback.
*   Never add a `postinstall` that puts a launcher earlier in resolution.

## 16.3 Deploy steps per platform/registry

### PyPI (`tui-lab`) — all three OSes, one job matrix

1.  `publish.yml` → `pypi` matrix (windows/linux/macOS runners).
2.  Each leg: `gh release download vX.Y.Z --pattern "*<target-triple>*"`
    → extract archives → `python tuilab/sdks/python/pack/pack_wheel.py
    --binary-dir bins --plat <win_amd64|manylinux_x86_64|macosx_11_0_arm64>`.
    The script stages `tuilab[.exe]` + `tuilab-mcp[.exe]` into
    `src/tuilab/_bin/`, builds via hatchling, retags the wheel
    `py3-none-<plat>`, and always cleans staging.
3.  `publish-pypi` merges the three wheels and publishes with
    `pypa/gh-action-pypi-publish` (OIDC trusted publishing, environment
    `pypi` — no token).
4.  Local check (Windows shown): pack from `tuilab/target/debug`, install the
    wheel in a fresh venv, `tuilab --version` + `find_binary()` both resolve
    under `site-packages/tuilab/_bin/`. Verified 2026-09-20.

### npm (`@tui-lab/cli*`, `@tui-lab/sdk`) — all three OSes, one job matrix

1.  `publish.yml` → `npm` matrix: download + extract as above, then
    `python tuilab/sdks/npm-cli/pack/pack_npm.py
    --bin <win32-x64|linux-x64|darwin-arm64>=bins`, producing the wrapper
    tarball plus the staged platform tarball in `dist/`.
2.  `publish-npm` publishes every tarball with `npm publish --provenance
    --access public` (OIDC, `id-token: write` — no token).
3.  `@tui-lab/sdk` is pure JS and resolves the engine via the installed
    `@tui-lab/cli-<platform>` package first. The hard dependency on
    `@tui-lab/cli` is **deferred to first publish** (adding it now would
    break `npm ci` against the empty registry); until then co-install:
    `npm i @tui-lab/sdk @tui-lab/cli`.
4.  Local check: `npm install` both tarballs → `.bin/tuilab --version`
    prints the engine version. Verified 2026-09-20.

### crates.io (Rust) — single Ubuntu job, strict dependency order

1.  All 12 crates carry `description.workspace` (crates.io requires it);
    `cargo publish --dry-run -p tui-lab-input` passes.
2.  `publish.yml` → `cargo` publishes in topological order (leaves first),
    sleeping 30s between crates for index propagation:
    `tui-lab-input tui-lab-protocol tui-lab-pty tui-lab-runtime
    tui-lab-terminal tui-lab-assertions` → `tui-lab-snapshots` →
    `tui-lab-core` → `tui-lab-reporter` →
    `tui-lab-cli tui-lab-mcp tui-lab-sdk`.
3.  Needs repo secret `CARGO_REGISTRY_TOKEN` (publish scope).
4.  Users then run `cargo install tui-lab-cli tui-lab-mcp` (package names;
    binaries are still `tuilab` / `tuilab-mcp`).

## 16.4 Maintainer runbook (first release)

1.  One-time setups: PyPI trusted publisher for `tui-lab` (workflow
    `publish.yml`, environment `pypi`); npm trusted publisher for the
    `@tui-lab` scope; `CARGO_REGISTRY_TOKEN` secret; GitHub Pages source =
    GitHub Actions (for the web guide).
2.  Bump all four versions (`tuilab/Cargo.toml` workspace, `pyproject.toml`,
    `sdks/javascript/package.json`, `sdks/npm-cli/package.json` +
    platform stubs) to `X.Y.Z` — `publish.yml` enforces equality with the tag.
3.  Green `main` → `git tag vX.Y.Z` → `git push origin vX.Y.Z`.
4.  `release.yml` builds archives + GitHub Release; its `published` event
    fires `publish.yml` (wheels → PyPI, tarballs → npm, crates → crates.io).
5.  Verify: fresh-machine `pip install tui-lab`, `npx @tui-lab/cli --version`,
    `cargo install tui-lab-cli` on each OS; then announce.

## 16.5 Files

*   `tuilab/sdks/python/src/tuilab/_cli.py` — console-script launchers.
*   `tuilab/sdks/python/pack/pack_wheel.py` — stage → build → retag → clean.
*   `tuilab/sdks/python/tests/test_launcher.py` — anti-loop regression tests.
*   `tuilab/sdks/npm-cli/{package.json,bin/,lib/,platforms/,pack/}` —
    wrapper, launchers, resolver, stubs, pack script.
*   `.github/workflows/publish.yml` — tag → registries pipeline.
