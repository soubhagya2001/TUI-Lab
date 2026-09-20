# @tui-lab/sdk (TypeScript)

Thin TypeScript client for [TUI Lab](../../../README.md): shells out to the
`tuilab` sidecar binary over JSON-lines (`tuilab proto`). Zero native PTY
code — one behavior across CLI, MCP, and SDKs.

```ts
import { TuiTest } from "@tui-lab/sdk";

const tui = await TuiTest.launch("./myapp");
try {
  await tui.expectText("Welcome");
  await tui.press("ENTER");
  await tui.expectText("Dashboard");
} finally {
  await tui.close();
}
```

```bash
npm run build
npm test
```

The binary is found via `TUILAB_BIN`, then `PATH`, then the workspace
`target/debug` layout. See `docs/09-sdk-integration-guide.md`.
