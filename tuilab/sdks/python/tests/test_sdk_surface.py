"""K1–K4 SDK surface: assert(), dropped options, idempotent close, fresh results."""

import asyncio
import os
import subprocess
import time
from pathlib import Path

import pytest

from tuilab import Runner, TuiLabError, TuiTest, find_binary
from tuilab import _read_results  # noqa: F401  (unit under test)


def fixture_bin() -> str:
    here = Path(__file__).resolve().parent.parent.parent.parent
    target = here / "tests" / "fixtures" / "ratatui-sample"
    subprocess.run(["cargo", "build"], cwd=target, check=True, capture_output=True)
    exe = "ratatui-sample.exe" if os.name == "nt" else "ratatui-sample"
    return str(target / "target" / "debug" / exe).replace("\\", "/")


async def _launch() -> TuiTest:
    tui = await TuiTest.launch(command=fixture_bin())
    await tui.expect_text("TUI-LAB-SAMPLE")
    return tui


def test_assert_condition_roundtrip() -> None:
    async def check() -> None:
        async with await _launch() as tui:
            passed = await tui.assert_({"type": "text_visible", "text": "TUI-LAB-SAMPLE"})
            assert passed["passed"] is True
            failed = await tui.assert_({"type": "text_visible", "text": "no-such-screen"})
            assert failed["passed"] is False
            with pytest.raises(TuiLabError):
                await tui.assert_({"type": "no-such-condition"})

    asyncio.run(check())


def test_screen_styled_returns_cells() -> None:
    async def check() -> None:
        async with await _launch() as tui:
            plain = await tui.screen()
            assert "text" in plain
            styled = await tui.screen(styled=True)
            assert isinstance(styled.get("cells"), list)
            assert styled["cells"], "styled screen carries cells"

    asyncio.run(check())


def test_expect_text_accepts_poll_ms() -> None:
    async def check() -> None:
        async with await _launch() as tui:
            screen = await tui.expect_text("TUI-LAB-SAMPLE", poll_ms=25)
            assert "TUI-LAB-SAMPLE" in screen

    asyncio.run(check())


def test_close_with_quit_is_clean_and_idempotent() -> None:
    async def check() -> None:
        tui = await _launch()
        first = await tui.close(quit="q")
        assert first.get("ok") is True
        second = await tui.close()
        assert second.get("ok") is True
        assert second.get("detail") == "already closed"

    asyncio.run(check())


def test_connection_double_close_never_hangs() -> None:
    from tuilab._proto import Connection

    async def check() -> None:
        conn = await Connection.spawn(find_binary())
        await asyncio.wait_for(conn.close(), timeout=15)
        await asyncio.wait_for(conn.close(), timeout=15)

    asyncio.run(check())


def test_read_results_rejects_stale_file(tmp_path, monkeypatch) -> None:
    monkeypatch.chdir(tmp_path)
    reports = tmp_path / "reports"
    reports.mkdir()
    target = reports / "results.json"
    target.write_text("[]")
    old = time.time() - 60
    os.utime(target, (old, old))
    with pytest.raises(TuiLabError, match="stale"):
        _read_results(time.time())
    target.write_text('[{"passed": true}]')
    assert _read_results(time.time() - 1) == [{"passed": True}]
    assert Runner is not None
