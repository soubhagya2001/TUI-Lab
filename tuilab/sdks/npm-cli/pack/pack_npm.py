"""Stage platform binaries and `npm pack` the CLI packages.

Usage (run from tuilab/sdks/npm-cli/):
    python pack/pack_npm.py --bin win32-x64=<dir> [--bin linux-x64=<dir> ...]

Each --bin stages tuilab[.exe]/tuilab-mcp[.exe] into platforms/<key>/, then
`npm pack`s the wrapper plus every staged platform package into dist/.
Unstaged platforms are skipped (their binaries come from CI matrix legs).
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent
PLATFORMS = HERE / "platforms"
DIST = HERE / "dist"


def stage(key: str, binary_dir: Path) -> None:
    dest = PLATFORMS / key
    if not dest.is_dir():
        raise SystemExit(f"unknown platform package: {key}")
    copied = 0
    for pattern in ("tuilab", "tuilab.exe", "tuilab-mcp", "tuilab-mcp.exe"):
        src = binary_dir / pattern
        if src.is_file():
            shutil.copy2(src, dest / src.name)
            copied += 1
    if copied == 0:
        raise SystemExit(f"no binaries found in {binary_dir}")
    print(f"staged {copied} binaries into {key}")


def pack(directory: Path) -> None:
    npm = shutil.which("npm") or ("npm.cmd" if os.name == "nt" else "npm")
    subprocess.run([npm, "pack", "--pack-destination", str(DIST)], cwd=directory, check=True)


def main() -> int:
    parser = argparse.ArgumentParser(description="Pack @tui-lab/cli npm packages.")
    parser.add_argument("--bin", action="append", default=[], metavar="KEY=DIR",
                        help="stage binaries, e.g. --bin win32-x64=../../target/debug")
    args = parser.parse_args()

    staged: list[str] = []
    for item in args.bin:
        key, _, directory = item.partition("=")
        if not key or not directory:
            raise SystemExit(f"bad --bin (want KEY=DIR): {item}")
        stage(key, Path(directory))
        staged.append(key)
    try:
        DIST.mkdir(parents=True, exist_ok=True)
        pack(HERE)
        for key in staged:
            pack(PLATFORMS / key)
        print(f"packages ready in {DIST}")
        return 0
    finally:
        for key in staged:
            for pattern in ("tuilab", "tuilab.exe", "tuilab-mcp", "tuilab-mcp.exe"):
                (PLATFORMS / key / pattern).unlink(missing_ok=True)


if __name__ == "__main__":
    raise SystemExit(main())
