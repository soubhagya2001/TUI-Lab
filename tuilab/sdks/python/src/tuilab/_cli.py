"""Console-script entry points: `tuilab` and `tuilab-mcp` on PATH.

Executes the bundled wheel binary (or PATH/workspace fallback) in-process
via os.execv, so signals and exit codes pass through untouched.
"""

from __future__ import annotations

import os
import sys

from ._proto import find_engine


def _exec(kind: str, argv: list[str]) -> int:
    binary = find_engine(kind)
    os.execv(str(binary), [str(binary), *argv])
    return 1  # unreachable; keeps type-checkers calm


def tuilab() -> int:
    """`tuilab` console script."""
    return _exec("tuilab", sys.argv[1:])


def tuilab_mcp() -> int:
    """`tuilab-mcp` console script."""
    return _exec("tuilab-mcp", sys.argv[1:])


if __name__ == "__main__":  # `python -m tuilab._cli …` escape hatch
    sys.exit(_exec("tuilab", sys.argv[1:]))
