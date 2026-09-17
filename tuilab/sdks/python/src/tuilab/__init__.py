"""Async TUI session over the sidecar (docs/09 §9.3 usage)."""

from __future__ import annotations

import asyncio
import json
import os
import subprocess
from pathlib import Path

from ._proto import Connection, TuiLabError, find_binary


def _check(reply: dict, action: str) -> dict:
    if not reply.get("ok", False):
        raise TuiLabError(f"{action} failed: {reply.get('error')}", reply)
    return reply


class TuiTest:
    """One application under test: launch → interact → assert → close."""

    def __init__(self, conn: Connection, session_id: str) -> None:
        self._conn = conn
        self.session_id = session_id

    @classmethod
    async def launch(
        cls,
        command: str,
        *,
        args: list[str] | None = None,
        cwd: str | None = None,
        env: dict[str, str] | None = None,
        width: int = 120,
        height: int = 40,
        binary: str | os.PathLike[str] | None = None,
    ) -> "TuiTest":
        """Launch an app under a fresh PTY; returns a live session."""
        conn = await Connection.spawn(binary)
        try:
            reply = await conn.request(
                {
                    "action": "launch",
                    "command": command,
                    "args": args or [],
                    "cwd": cwd,
                    "terminal": {"width": width, "height": height},
                    "env": env or {},
                }
            )
        except Exception:
            await conn.close()
            raise
        try:
            _check(reply, "launch")
        except Exception:
            await conn.close()
            raise
        return cls(conn, reply["session_id"])

    async def __aenter__(self) -> "TuiTest":
        return self

    async def __aexit__(self, *exc: object) -> None:
        await self.close()

    async def press(self, key: str) -> bool:
        """Send a named key; returns whether the screen changed."""
        reply = _check(
            await self._conn.request(
                {"action": "press", "session_id": self.session_id, "key": key}
            ),
            "press",
        )
        return bool(reply.get("screen_changed", False))

    async def type(self, text: str, *, sensitive: bool = False) -> None:
        """Type text verbatim (sensitive redacts it from logs)."""
        _check(
            await self._conn.request(
                {
                    "action": "type",
                    "session_id": self.session_id,
                    "text": text,
                    "sensitive": sensitive,
                }
            ),
            "type",
        )

    async def screen(self) -> dict:
        """Current screen grid (text, cursor, dimensions)."""
        return _check(
            await self._conn.request(
                {"action": "screen", "session_id": self.session_id}
            ),
            "screen",
        )

    async def expect_text(
        self, text: str, *, timeout_ms: int = 10_000, regex: bool = False
    ) -> str:
        """Poll until text is visible; raise with the last screen on timeout."""
        reply = _check(
            await self._conn.request(
                {
                    "action": "wait_for_text",
                    "session_id": self.session_id,
                    "text": text,
                    "regex": regex,
                    "timeout_ms": timeout_ms,
                }
            ),
            "wait_for_text",
        )
        if not reply.get("found"):
            raise TuiLabError(f"timed out waiting for {text!r}", reply)
        return str(reply.get("screen", ""))

    async def expect_not_text(self, text: str) -> None:
        """Assert text is absent from the current screen."""
        screen = await self.screen()
        if text in screen.get("text", ""):
            raise TuiLabError(f"unexpected visible text {text!r}", screen)

    async def snapshot(self, name: str) -> dict:
        """Capture a named snapshot (golden written on first use)."""
        return _check(
            await self._conn.request(
                {"action": "snapshot", "session_id": self.session_id, "name": name}
            ),
            "snapshot",
        )

    async def resize(self, width: int, height: int) -> None:
        """Resize the terminal."""
        _check(
            await self._conn.request(
                {
                    "action": "resize",
                    "session_id": self.session_id,
                    "width": width,
                    "height": height,
                }
            ),
            "resize",
        )

    async def close(self) -> dict:
        """Close the session and reap the child."""
        try:
            reply = _check(
                await self._conn.request(
                    {"action": "close", "session_id": self.session_id}
                ),
                "close",
            )
            return reply
        finally:
            await self._conn.close()


class Runner:
    """YAML suites through `tuilab run` (canonical format, docs/05)."""

    @staticmethod
    async def run(
        test_file: str | os.PathLike[str],
        *,
        binary: str | os.PathLike[str] | None = None,
    ) -> list[dict]:
        """Run a suite file; return parsed results (raise on infra failures)."""
        proc = await asyncio.create_subprocess_exec(
            str(find_binary(binary)),
            "run",
            str(test_file),
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.STDOUT,
        )
        out, _ = await proc.communicate()
        code = proc.returncode or 0
        if code not in (0, 1):
            raise TuiLabError(
                f"tuilab run exited {code}: {out.decode(errors='replace')[-2000:]}"
            )
        # Results land next to the invoker's CWD (reports/results.json).
        results = Path("reports/results.json")
        if not results.is_file():
            raise TuiLabError("tuilab run produced no reports/results.json")
        return json.loads(results.read_text())


__all__ = ["Connection", "Runner", "TuiLabError", "TuiTest", "find_binary"]
