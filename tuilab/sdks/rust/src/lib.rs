//! Thin Rust client for TUI Lab: shells out to the `tuilab` sidecar binary
//! over JSON-lines (`tuilab proto`). Zero native PTY code — one behavior
//! across CLI, MCP, and SDKs.
//!
//! Native core linking is deferred by DECIDED rule (see `docs/09`): this
//! crate must stay a sidecar client. Revisit only through a grill round.
//!
//! ```no_run
//! use tui_lab_sdk::{LaunchOptions, TuiTest};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), tui_lab_sdk::TuiLabError> {
//!     let mut tui = TuiTest::launch("./myapp", LaunchOptions::new()).await?;
//!     tui.expect_text("Welcome", 10_000, false).await?;
//!     tui.press("ENTER").await?;
//!     tui.expect_text("Dashboard", 10_000, false).await?;
//!     tui.close().await?;
//!     Ok(())
//! }
//! ```

pub mod binary;
pub mod error;
pub mod proto;
pub mod session;

pub use binary::find_binary;
pub use error::TuiLabError;
pub use proto::{Connection, LaunchOptions};
pub use session::{Runner, TuiTest};
