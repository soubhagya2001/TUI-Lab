"""Build a platform wheel embedding the prebuilt TUI Lab binaries.

Usage (run from tuilab/sdks/python/):
    python pack/pack_wheel.py --binary-dir <dir-with-tuilab-bins> --plat <tag>

Example:
    python pack/pack_wheel.py --binary-dir ../../target/debug --plat win_amd64

Flow: stage binaries into src/tuilab/_bin/ → `python -m build --wheel` →
retag py3-none-any to py3-none-<plat> via `wheel tags` → clean staging.

Requires (pack-time only): `build`, `wheel`.
"""

from __future__ import annotations

import argparse
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent
BIN_STAGING = HERE / "src" / "tuilab" / "_bin"
BINARIES = ("tuilab", "tuilab-mcp")


def project_version() -> str:
    """Single source of truth: `pyproject.toml` (lockstep with the tag)."""
    text = (HERE / "pyproject.toml").read_text()
    match = re.search(r'^version = "([^"]+)"', text, re.MULTILINE)
    if not match:
        raise SystemExit("no version in pyproject.toml")
    return match.group(1)


def stage(binary_dir: Path) -> list[Path]:
    BIN_STAGING.mkdir(parents=True, exist_ok=True)
    staged: list[Path] = []
    for kind in BINARIES:
        src = binary_dir / kind
        if not src.is_file():
            src = binary_dir / f"{kind}.exe"
        if not src.is_file():
            raise SystemExit(f"binary not found for {kind} in {binary_dir}")
        dest = BIN_STAGING / src.name
        shutil.copy2(src, dest)
        # K5: the exec bit dies on Windows checkouts — wheels must stay
        # executable on Unix targets regardless of the staging host.
        os.chmod(dest, 0o755)
        staged.append(dest)
    return staged


def main() -> int:
    parser = argparse.ArgumentParser(description="Pack a platform tui-lab wheel.")
    parser.add_argument("--binary-dir", required=True, help="dir holding tuilab/tuilab-mcp")
    parser.add_argument("--plat", required=True, help="platform tag, e.g. win_amd64")
    parser.add_argument("--outdir", default=str(HERE / "dist"))
    args = parser.parse_args()

    staged = stage(Path(args.binary_dir))
    print(f"staged: {[p.name for p in staged]}")
    try:
        subprocess.run(
            [sys.executable, "-m", "build", "--wheel", "--outdir", args.outdir],
            cwd=HERE,
            check=True,
        )
        # K5: select by exact version, never lexicographic-last — a stale
        # wheel from another platform must fail loudly, not ship silently.
        want = f"tui_lab-{project_version()}-py3-none-any.whl"
        matches = [p for p in Path(args.outdir).glob("tui_lab-*.whl") if p.name == want]
        if len(matches) != 1:
            raise SystemExit(
                f"expected exactly one {want} in {args.outdir}, found: "
                f"{[p.name for p in Path(args.outdir).glob('tui_lab-*.whl')]}"
            )
        wheel = matches[0]
        subprocess.run(
            [
                sys.executable, "-m", "wheel", "tags",
                "--python-tag", "py3",
                "--abi-tag", "none",
                "--platform-tag", args.plat,
                "--remove",
                str(wheel),
            ],
            check=True,
        )
        print(f"wheel ready in {args.outdir}")
        return 0
    finally:
        shutil.rmtree(BIN_STAGING, ignore_errors=True)


if __name__ == "__main__":
    raise SystemExit(main())
