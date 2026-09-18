"""Async SDK flow against the Ratatui fixture (mirrors runtime_smoke)."""

import asyncio
import os
import subprocess
from pathlib import Path

import pytest

from tuilab import Runner, TuiLabError, TuiTest, find_binary

WORKSPACE = Path(__file__).resolve().parent.parent.parent.parent
FIXTURE_DIR = WORKSPACE / "tests" / "fixtures" / "ratatui-sample"
EXE = "ratatui-sample.exe" if os.name == "nt" else "ratatui-sample"


def fixture_bin() -> str:
    subprocess.run(
        ["cargo", "build"],
        cwd=FIXTURE_DIR,
        check=True,
        capture_output=True,
    )
    # Forward slashes: safe inside YAML double-quoted scalars on Windows.
    return str(FIXTURE_DIR / "target" / "debug" / EXE).replace("\\", "/")


async def _flow() -> None:
    async with await TuiTest.launch(command=fixture_bin()) as tui:
        await tui.expect_text("TUI-LAB-SAMPLE")
        await tui.press("DOWN")
        await tui.press("ENTER")
        screen = await tui.expect_text("Selected: beta-chair")
        assert "beta-chair" in screen
        await tui.resize(80, 24)
        await tui.press("ESC")
        await tui.expect_text("TUI-LAB-SAMPLE")
        await tui.press("/")
        await tui.type("table")
        await tui.expect_text("Search: table")
        await tui.expect_not_text("beta-chair")
        snap = await tui.snapshot("sdk-proof")
        assert snap["ok"] is True


def test_fixture_flow() -> None:
    asyncio.run(_flow())


def test_unknown_key_raises() -> None:
    async def check() -> None:
        async with await TuiTest.launch(command=fixture_bin()) as tui:
            await tui.expect_text("TUI-LAB-SAMPLE")
            with pytest.raises(TuiLabError):
                await tui.press("F13")

    asyncio.run(check())


def test_wait_timeout_raises_with_screen() -> None:
    async def check() -> None:
        async with await TuiTest.launch(command=fixture_bin()) as tui:
            with pytest.raises(TuiLabError) as exc:
                await tui.expect_text("no-such-screen", timeout_ms=500)
            assert "screen" in (exc.value.detail or {})

    asyncio.run(check())


def test_runner_runs_yaml(tmp_path) -> None:
    suite = tmp_path / "mini.yaml"
    suite.write_text(
        f"""schema: tui-lab/v1
name: mini
application:
  command: "{fixture_bin()}"
steps:
  - wait_for_text:
      text: "TUI-LAB-SAMPLE"
  - press: q
assertions:
  - exit_code: 0
"""
    )
    results = asyncio.run(Runner.run(suite, binary=find_binary()))
    assert isinstance(results, list) and results
    assert all(suite["passed"] for suite in results)


def test_find_binary_resolves() -> None:
    assert find_binary().is_file()
