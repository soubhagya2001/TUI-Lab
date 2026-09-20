//! Locate the `tuilab` binary: explicit → TUILAB_BIN → PATH → workspace.

use std::path::{Path, PathBuf};

use crate::error::TuiLabError;

/// Resolve the `tuilab` binary.
pub fn find_binary(explicit: Option<&Path>) -> Result<PathBuf, TuiLabError> {
    if let Some(path) = explicit {
        if path.is_file() {
            return Ok(path.to_path_buf());
        }
        return Err(TuiLabError::new(format!(
            "tuilab binary not found: {}",
            path.display()
        )));
    }
    if let Ok(from_env) = std::env::var("TUILAB_BIN") {
        let path = PathBuf::from(&from_env);
        if path.is_file() {
            return Ok(path);
        }
    }
    if let Some(path) = search_path() {
        return Ok(path);
    }
    // src files live at sdks/rust/src → walk up to tuilab/ → target/debug.
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let candidate = PathBuf::from(manifest)
            .join("..")
            .join("..")
            .join("target")
            .join("debug")
            .join(format!("tuilab{}", std::env::consts::EXE_SUFFIX));
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(TuiLabError::new(
        "tuilab binary not found: set TUILAB_BIN, add it to PATH, \
         or build the workspace (cargo build -p tui-lab-cli)",
    ))
}

/// Search `PATH` for the binary (plus `.exe` on Windows).
fn search_path() -> Option<PathBuf> {
    let name = format!("tuilab{}", std::env::consts::EXE_SUFFIX);
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join(&name))
            .find(|candidate| candidate.is_file())
    })
}
