"""Set every registry version in lockstep (K5).

Usage (run from repo root):
    python tuilab/pack/bump_versions.py 0.2.0
    python tuilab/pack/bump_versions.py --check

Updates all 7 version sites checked by the publish.yml versions gate:
workspace Cargo.toml, Python pyproject.toml, JS SDK package.json,
npm-cli wrapper package.json, and the 3 npm platform stubs.

`--check` is the PR-time gate: it reports every site whose version has
drifted and exits non-zero without writing anything.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent


def set_cargo(path: Path, version: str) -> None:
    text = path.read_text()
    updated, count = re.subn(
        r'(?m)^version = "[^"]+"', f'version = "{version}"', text, count=1
    )
    if count != 1:
        raise SystemExit(f"no version line in {path}")
    path.write_text(updated)


def set_pyproject(path: Path, version: str) -> None:
    text = path.read_text()
    updated, count = re.subn(
        r'(?m)^version = "[^"]+"', f'version = "{version}"', text, count=1
    )
    if count != 1:
        raise SystemExit(f"no version line in {path}")
    path.write_text(updated)


def set_npm(path: Path, version: str) -> None:
    # Surgical string replacement: a json round-trip would reformat arrays
    # and flood the diff. Sibling optional deps move together.
    text = path.read_text()
    updated, count = re.subn(
        r'(?m)^(\s*"version": ")[^"]+(")', rf"\g<1>{version}\g<2>", text, count=1
    )
    if count != 1:
        raise SystemExit(f"no version field in {path}")
    for sibling in ("@tui-lab/cli", "@tui-lab/cli-win32-x64", "@tui-lab/cli-linux-x64",
                    "@tui-lab/cli-darwin-arm64"):
        updated = re.sub(
            rf'("{re.escape(sibling)}": ")[^"]+(")', rf"\g<1>{version}\g<2>", updated
        )
    path.write_text(updated)


def version_sites() -> list[tuple[str, Path]]:
    npm_cli = ROOT / "tuilab" / "sdks" / "npm-cli"
    return [
        ("tuilab/Cargo.toml", ROOT / "tuilab" / "Cargo.toml"),
        ("sdks/python/pyproject.toml", ROOT / "tuilab" / "sdks" / "python" / "pyproject.toml"),
        ("sdks/javascript/package.json", ROOT / "tuilab" / "sdks" / "javascript" / "package.json"),
        ("sdks/npm-cli/package.json", npm_cli / "package.json"),
        *[
            (f"sdks/npm-cli/platforms/{plat}/package.json", npm_cli / "platforms" / plat / "package.json")
            for plat in ("win32-x64", "linux-x64", "darwin-arm64")
        ],
    ]


def read_version(path: Path) -> str:
    text = path.read_text()
    if path.suffix == ".toml":
        match = re.search(r'(?m)^version = "([^"]+)"', text)
    else:
        match = re.search(r'(?m)^\s*"version": "([^"]+)"', text)
    if not match:
        raise SystemExit(f"no version field in {path}")
    return match.group(1)


def check() -> int:
    """Fail when any of the 7 sites disagrees with the rest (PR gate)."""
    found = [(label, path, read_version(path)) for label, path in version_sites()]
    wanted = found[0][2]
    drifted = [(label, version) for label, _, version in found if version != wanted]
    if drifted:
        print(f"version drift: {found[0][0]} is {wanted}, others disagree:", file=sys.stderr)
        for label, version in drifted:
            print(f"  {label}: {version}", file=sys.stderr)
        print("fix: python tuilab/pack/bump_versions.py <version>", file=sys.stderr)
        return 1
    print(f"all 7 version sites agree on {wanted}")
    return 0


def main() -> int:
    if sys.argv[1:] == ["--check"]:
        return check()
    if len(sys.argv) != 2 or not re.fullmatch(r"\d+\.\d+\.\d+", sys.argv[1]):
        raise SystemExit("usage: bump_versions.py X.Y.Z | --check")
    version = sys.argv[1]
    for _, path in version_sites():
        if path.suffix == ".toml":
            (set_cargo if path.name == "Cargo.toml" else set_pyproject)(path, version)
        else:
            set_npm(path, version)
    print(f"all 7 version sites set to {version}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
