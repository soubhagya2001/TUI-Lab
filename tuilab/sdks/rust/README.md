# tui-lab-sdk (Rust)

Thin Rust client for [TUI Lab](../../../README.md): shells out to the
`tuilab` sidecar binary over JSON-lines (`tuilab proto`). Zero native PTY
code — one behavior across CLI, MCP, and SDKs. Native core linking is
deferred by DECIDED rule (see `docs/09`).

```rust
let mut tui = tui_lab_sdk::TuiTest::launch(
    "./myapp",
    tui_lab_sdk::LaunchOptions::new(),
)
.await?;
tui.expect_text("Welcome", 10_000, false).await?;
tui.press("ENTER").await?;
tui.expect_text("Dashboard", 10_000, false).await?;
tui.close().await?;
```

The binary is found via `TUILAB_BIN`, then `PATH`, then the workspace
`target/debug` layout. See `docs/09-sdk-integration-guide.md`.
