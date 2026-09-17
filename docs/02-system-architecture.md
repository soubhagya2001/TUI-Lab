# 02 — System Architecture

## 2.1 Three-layer model

```
+---------------- Integration Layer ------------------+
|  CLI (tuilab)  MCP Server  SDKs  REST/API(future) CI |
+------------------------+--------------------------+
                         |
+---------------- TUI Lab Test Protocol --------------+
| Launch | Input | Wait | Inspect | Assert | Snapshot | Exit |
+------------------------+--------------------------+
                         |
+----------------- Universal Runtime -----------------+
| Process Mgr, PTY/ConPTY, Terminal Emulator,         |
| Input Injection, Screen Buffer, Assertion Engine,   |
| Artifact Collector                                  |
+------------------------+--------------------------+
                         |
                  Any TUI Application
  (Rust / Go / Python / C / Java / Node / Bash — any)
```

## 2.2 Component breakdown

```
                    CLI / Desktop(future) / API
                                   |
                          Test Orchestrator
                                   |
        +------------+-------------+-------------+------------+
        |                          |                          |
  Test Parser              Action Engine            Assertion Engine
  (YAML/JSON -> steps)     (press/type/resize)      (text/screen/proc)
                                   |
                           Runtime Supervisor
                           (timeouts, retries, lifecycle)
                                   |
                           PTY Abstraction
                         (Unix PTY / Win ConPTY)
                                   |
                           Terminal Emulator
                         (ANSI parser + grid buffer)
                                   |
                            TUI Application
```

## 2.3 Key components

| Component | Responsibility |
|-----------|----------------|
| Test Definition Layer | YAML/JSON tests + TS/Rust SDKs (thin wrappers, Phase 5) |
| Test Execution Engine | Action executor, assertion engine, wait/retry/timeout, lifecycle |
| Terminal Runtime Layer | PTY manager, process manager, emulator, input injection, output capture |
| Reporting Layer | HTML report, JSON/JUnit XML, screenshots/snapshots, logs/replay |

## 2.4 Core runtime state

```rust
pub struct TestContext {
    pub process_id: u32,
    pub terminal: VirtualTerminal,
    pub start_time: std::time::Instant,
    pub variables: std::collections::HashMap<String, String>,
    pub artifacts: Vec<Artifact>,
}
```

## 2.5 Execution pipeline (per test case)

```
Parse YAML -> Create TestContext -> Launch PTY -> Start App
  -> Read terminal output -> Update virtual terminal
  -> Execute action -> Wait / Assert -> Capture artifacts
  -> Cleanup process -> Generate result
```

All transports (CLI local, MCP stdio, future HTTP/WS) funnel into the same `tui-lab-core` engine. No separate logic per frontend.
