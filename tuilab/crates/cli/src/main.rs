//! `tuilab` CLI: init/run/report/record/proto (docs/07, docs/09).

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use tui_lab_cli::{commands, proto};

/// TUI Lab: black-box testing for terminal applications.
#[derive(Debug, Parser)]
#[command(name = "tuilab", version)]
struct Cli {
    /// Subcommand.
    #[command(subcommand)]
    command: Commands,
}

/// Available subcommands.
#[derive(Debug, Subcommand)]
enum Commands {
    /// Scaffold tuilab.yaml + tests/smoke.yaml.
    Init,
    /// Run suites sequentially (defaults to the configured tests dir).
    Run {
        /// Suite file or directory.
        path: Option<PathBuf>,
        /// Terminal override, e.g. 120x40.
        #[arg(long, value_parser = parse_terminal)]
        terminal: Option<(u16, u16)>,
        /// Print the full failure bundle on failure.
        #[arg(long)]
        debug: bool,
        /// Step-through mode (v2).
        #[arg(long)]
        step: bool,
    },
    /// Re-render stored results (junit for now).
    Report {
        /// Output format.
        #[arg(long, default_value = "junit")]
        format: String,
        /// Output file.
        #[arg(long)]
        out: PathBuf,
        /// Stored results to render.
        #[arg(long, default_value = "reports/results.json")]
        results: PathBuf,
    },
    /// Record an interactive session to a YAML suite.
    Record {
        /// Binary to launch and record.
        #[arg(long)]
        command: Option<String>,
        /// Extra arguments for the binary (repeatable).
        #[arg(long)]
        arg: Vec<String>,
        /// Output YAML path (default: <command>-record.yaml).
        #[arg(long)]
        out: Option<PathBuf>,
        /// Terminal override, e.g. 120x40.
        #[arg(long, value_parser = parse_terminal)]
        terminal: Option<(u16, u16)>,
    },
    /// JSON-lines engine mode for SDK sidecars (docs/09).
    Proto,
}

/// Parse `120x40` terminal geometry.
fn parse_terminal(raw: &str) -> Result<(u16, u16), String> {
    let (width, height) = raw
        .split_once('x')
        .ok_or_else(|| format!("expected WxH, got {raw:?}"))?;
    let width: u16 = width.parse().map_err(|_| format!("bad width in {raw:?}"))?;
    let height: u16 = height
        .parse()
        .map_err(|_| format!("bad height in {raw:?}"))?;
    Ok((width, height))
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let cli = Cli::parse();
    let code = match cli.command {
        Commands::Init => {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            commands::init(&cwd)
        }
        Commands::Run {
            path,
            terminal,
            debug,
            step,
        } => commands::run(path.as_deref(), terminal, debug, step).await,
        Commands::Report {
            format,
            out,
            results,
        } => commands::report(&format, &out, &results),
        Commands::Record {
            command,
            arg,
            out,
            terminal,
        } => commands::record(command, arg, out, terminal),
        Commands::Proto => proto::serve().await,
    };
    std::process::exit(code);
}
