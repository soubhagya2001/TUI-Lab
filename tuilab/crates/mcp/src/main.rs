//! `tuilab-mcp` server: 9 `tui_*` tools over stdio (docs/08).
//!
//! Phase 0 scaffold — `rmcp` wiring arrives in Phase 4.

mod constants;
mod utils;

use constants::{FORBIDDEN_COMMAND, MAX_SESSIONS, SESSION_IDLE_SECS};

fn main() {
    println!("tuilab-mcp 0.1.0 (Phase 0 scaffold)");
    println!(
        "first session: {} (max {MAX_SESSIONS}, idle {SESSION_IDLE_SECS}s, deny: {FORBIDDEN_COMMAND})",
        utils::session_id(1)
    );
}
