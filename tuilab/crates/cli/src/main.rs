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
        /// Parallel slots (default: tuilab.yaml `parallel`).
        #[arg(long)]
        parallel: Option<usize>,
        /// Step-through mode (v2).
        #[arg(long)]
        step: bool,
        /// Shard selection N/M: run only the Nth slice of M (both 1-based).
        #[arg(long, value_parser = commands::parse_shard)]
        shard: Option<(usize, usize)>,
        /// Rerun failed suites up to N extra times (flakes get more chances).
        #[arg(long, default_value_t = 0)]
        retries: usize,
        /// Run only suites carrying any of these tags (comma-separated).
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
        /// Trace capture: `always` writes a trace.zip per suite, `never`
        /// disables; absent keeps retain-on-failure.
        #[arg(long, value_parser = ["always", "never"])]
        trace: Option<String>,
        /// Run every suite once per geometry, e.g. `80x24,120x40`.
        #[arg(long, value_delimiter = ',', value_parser = parse_terminal)]
        resize_matrix: Vec<(u16, u16)>,
        /// Write every report artifact under this directory instead of
        /// `reports/` (results.json, junit, html, history, traces,
        /// attachments) — keeps concurrent runs from clobbering each other.
        #[arg(long, value_name = "DIR")]
        output_dir: Option<PathBuf>,
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
        /// Output path (default: <command>-record.<ext for --target>).
        #[arg(long)]
        out: Option<PathBuf>,
        /// Terminal override, e.g. 120x40.
        #[arg(long, value_parser = parse_terminal)]
        terminal: Option<(u16, u16)>,
        /// Emit SDK code instead of YAML: python|js|rust (default yaml).
        #[arg(long, value_parser = tui_lab_cli::emit::Target::parse)]
        target: Option<tui_lab_cli::emit::Target>,
    },
    /// JSON-lines engine mode for SDK sidecars (docs/09).
    Proto,
    /// Render a trace.zip timeline, or replay its raw bytes (`--replay`)
    /// / recorded inputs (`--replay-input`).
    Trace {
        /// Trace archive from a run.
        zip: PathBuf,
        /// Stream raw PTY bytes to stdout with original pacing.
        #[arg(long)]
        replay: bool,
        /// Stream recorded input bytes with original pacing (P5-E2).
        #[arg(long)]
        replay_input: bool,
    },
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
            parallel,
            shard,
            retries,
            tags,
            trace,
            resize_matrix,
            output_dir,
        } => {
            commands::run(commands::RunArgs {
                path: path.as_deref(),
                terminal_override: terminal,
                debug,
                step_mode: step,
                parallel,
                shard,
                retries,
                tags: &tags,
                trace: trace.as_deref(),
                resize_matrix: &resize_matrix,
                output_dir: output_dir.as_deref(),
            })
            .await
        }
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
            target,
        } => commands::record(
            command,
            arg,
            out,
            terminal,
            target.unwrap_or(tui_lab_cli::emit::Target::Yaml),
        ),
        Commands::Proto => proto::serve().await,
        Commands::Trace {
            zip,
            replay,
            replay_input,
        } => commands::trace(&zip, replay, replay_input),
    };
    std::process::exit(code);
}
