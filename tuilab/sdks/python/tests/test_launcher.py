"""Launcher safety: entry points must exec a real engine, never themselves.

Regression test for the self-exec loop: once `[project.scripts]` installs
`tuilab`/`tuilab-mcp` launchers on PATH, naive PATH-first resolution spawns
the launcher as the engine and hangs. `find_engine` (used by the launchers)
skips PATH entirely; `find_binary` prefers bundle/workspace over PATH.
"""

import os
import subprocess
import sys

from tuilab import find_binary
from tuilab._proto import find_engine


def test_find_engine_skips_path_stub(tmp_path, monkeypatch) -> None:
    stub = tmp_path / ("tuilab.exe" if os.name == "nt" else "tuilab")
    stub.write_bytes(b"not an engine")
    monkeypatch.setenv("PATH", str(tmp_path), prepend=os.pathsep)
    monkeypatch.delenv("TUILAB_BIN", raising=False)
    resolved = find_engine()
    assert resolved != stub
    assert resolved.is_file()


def test_find_binary_prefers_workspace_over_path_stub(tmp_path, monkeypatch) -> None:
    stub = tmp_path / ("tuilab.exe" if os.name == "nt" else "tuilab")
    stub.write_bytes(b"not an engine")
    monkeypatch.setenv("PATH", str(tmp_path), prepend=os.pathsep)
    monkeypatch.delenv("TUILAB_BIN", raising=False)
    assert find_binary() != stub


def test_console_module_reports_engine_version() -> None:
    proc = subprocess.run(
        [sys.executable, "-m", "tuilab._cli", "--version"],
        capture_output=True,
        text=True,
        timeout=60,
    )
    assert proc.returncode == 0, proc.stderr[-2000:]
    assert "tuilab" in proc.stdout.lower()
