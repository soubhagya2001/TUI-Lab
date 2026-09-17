//! `tuilab-mcp` server: 9 `tui_*` tools over stdio (docs/08).
//!
//! Tracing goes to stderr — stdout belongs to the MCP transport.

use std::path::PathBuf;

use rmcp::ServiceExt;
use tui_lab_mcp::handler::TuiLabHandler;
use tui_lab_mcp::security::load_allowlist;
#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let allow = match load_allowlist(&root) {
        Ok(allow) => allow,
        Err(e) => {
            eprintln!("config error: {e}");
            std::process::exit(2);
        }
    };
    tracing::info!(root = %root.display(), "tuilab-mcp serving over stdio");

    let handler = TuiLabHandler::new(root, allow);
    match handler.serve(rmcp::transport::stdio()).await {
        Ok(running) => {
            if let Err(e) = running.waiting().await {
                eprintln!("server error: {e}");
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("serve error: {e}");
            std::process::exit(1);
        }
    }
}
