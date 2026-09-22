"""Set every registry version in lockstep (K5).

Usage (run from repo root):
    python tuilab/pack/bump_versions.py 0.2.0

Updates all 7 version sites checked by the publish.yml versions gate:
workspace Cargo.toml, Python pyproject.toml, JS SDK package.json,
npm-cli wrapper package.json, and the 3 npm platform stubs.
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


def main() -> int:
    if len(sys.argv) != 2 or not re.fullmatch(r"\d+\.\d+\.\d+", sys.argv[1]):
        raise SystemExit("usage: bump_versions.py X.Y.Z")
    version = sys.argv[1]
    set_cargo(ROOT / "tuilab" / "Cargo.toml", version)
    set_pyproject(ROOT / "tuilab" / "sdks" / "python" / "pyproject.toml", version)
    set_npm(ROOT / "tuilab" / "sdks" / "javascript" / "package.json", version)
    npm_cli = ROOT / "tuilab" / "sdks" / "npm-cli"
    set_npm(npm_cli / "package.json", version)
    for plat in ("win32-x64", "linux-x64", "darwin-arm64"):
        set_npm(npm_cli / "platforms" / plat / "package.json", version)
    print(f"all 7 version sites set to {version}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
