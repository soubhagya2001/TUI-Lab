//! `tuilab` CLI: init/run/record/report (docs/07).
//!
//! Phase 0 scaffold — `clap` wiring arrives in Phase 3.

mod constants;
mod utils;

use constants::{CONFIG_FILE, EXIT_OK, TESTS_DIR};

fn main() {
    println!("tuilab 0.1.0 (Phase 0 scaffold)");
    println!(
        "config: {CONFIG_FILE}, tests: {TESTS_DIR} — {}",
        utils::exit_message(EXIT_OK)
    );
}
