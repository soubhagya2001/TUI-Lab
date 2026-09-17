# tui-lab (Python)

Thin Python client for [TUI Lab](../../../README.md): shells out to the
`tuilab` sidecar binary over JSON-lines (`tuilab proto`). Zero native PTY
code — one behavior across CLI, MCP, and SDKs.

```python
import asyncio
from tuilab import TuiTest

async def main():
    async with await TuiTest.launch(command="python app.py") as tui:
        await tui.expect_text("Welcome")
        await tui.press("ENTER")
        await tui.expect_text("Dashboard")

asyncio.run(main())
```

The binary is found via `TUILAB_BIN`, then `PATH`, then the workspace
`target/debug` layout. See `docs/09-sdk-integration-guide.md`.
