"""JSON-lines sidecar transport: one action per stdin line, one reply out.

Speaks the `tuilab proto` contract pinned by
`tuilab/crates/cli/tests/proto_roundtrip.rs` — an engine change that breaks
SDKs fails there first.
"""

from __future__ import annotations

import asyncio
import json
import os
import shutil
from pathlib import Path


class TuiLabError(Exception):
    """Engine or transport failure, with step/session context attached."""

    def __init__(self, message: str, detail: dict | None = None) -> None:
        super().__init__(message)
        self.detail = detail or {}


def find_binary(explicit: str | os.PathLike[str] | None = None) -> Path:
    """Resolve the `tuilab` binary: explicit → TUILAB_BIN → PATH → workspace."""
    if explicit:
        candidate = Path(explicit)
        if candidate.is_file():
            return candidate
        raise TuiLabError(f"tuilab binary not found: {explicit}")
    env = os.environ.get("TUILAB_BIN")
    if env and Path(env).is_file():
        return Path(env)
    on_path = shutil.which("tuilab")
    if on_path:
        return Path(on_path)
    workspace = (
        Path(__file__).resolve().parent.parent.parent.parent.parent
        / "target"
        / "debug"
        / ("tuilab.exe" if os.name == "nt" else "tuilab")
    )
    if workspace.is_file():
        return workspace
    raise TuiLabError(
        "tuilab binary not found: set TUILAB_BIN, add it to PATH, "
        "or build the workspace (cargo build -p tui-lab-cli)"
    )


class Connection:
    """One long-lived `tuilab proto` process."""

    def __init__(self, proc: asyncio.subprocess.Process) -> None:
        self._proc = proc

    @classmethod
    async def spawn(
        cls, binary: str | os.PathLike[str] | None = None
    ) -> "Connection":
        proc = await asyncio.create_subprocess_exec(
            str(find_binary(binary)),
            "proto",
            stdin=asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.PIPE,
            stderr=None,
        )
        return cls(proc)

    async def request(self, action: dict) -> dict:
        """Send one action, return the reply dict (raises on transport errors)."""
        assert self._proc.stdin and self._proc.stdout
        self._proc.stdin.write((json.dumps(action) + "\n").encode())
        await self._proc.stdin.drain()
        line = await asyncio.wait_for(self._proc.stdout.readline(), timeout=60)
        if not line:
            raise TuiLabError("tuilab proto closed stdout")
        try:
            return json.loads(line)
        except json.JSONDecodeError as exc:
            raise TuiLabError(f"proto returned non-JSON: {line!r}") from exc

    async def close(self) -> None:
        """EOF the engine and reap the process."""
        if self._proc.stdin:
            self._proc.stdin.close()
        try:
            await asyncio.wait_for(self._proc.wait(), timeout=10)
        except asyncio.TimeoutError:
            self._proc.kill()
