//! MCP helpers (shared small utilities for this crate).
//!
//! Session ids come from `tui-lab-core` so every frontend agrees.

#[cfg(test)]
mod tests {
    #[test]
    fn core_session_ids_are_zero_padded() {
        assert_eq!(tui_lab_core::utils::session_id(7), "sess_007");
    }
}
