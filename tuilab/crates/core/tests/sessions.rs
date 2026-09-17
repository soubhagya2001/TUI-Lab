//! Session registry: caps, lookup, idle reaping (docs/08 §8.4).

use std::time::Duration;

use tui_lab_core::{NewSession, SessionRegistry};
use tui_lab_pty::SpawnOptions;

/// Short-lived shell for registry mechanics (no grid assertions here).
fn shell_spawn() -> SpawnOptions {
    #[cfg(windows)]
    {
        SpawnOptions {
            command: "cmd".to_string(),
            args: vec!["/Q".to_string()],
            ..SpawnOptions::default()
        }
    }
    #[cfg(not(windows))]
    {
        SpawnOptions {
            command: "cat".to_string(),
            ..SpawnOptions::default()
        }
    }
}

#[test]
fn spawn_returns_sequential_ids() {
    let mut registry = SessionRegistry::new();
    let first = registry
        .spawn(NewSession::new(shell_spawn()))
        .expect("spawn 1");
    let second = registry
        .spawn(NewSession::new(shell_spawn()))
        .expect("spawn 2");
    assert_eq!(first, "sess_001");
    assert_eq!(second, "sess_002");
    assert_eq!(registry.len(), 2);
    registry.remove(&first).expect("remove");
    registry.remove(&second).expect("remove");
    assert!(registry.is_empty());
}

#[test]
fn cap_is_enforced_without_orphans() {
    let mut registry = SessionRegistry::with_cap(2);
    let first = registry
        .spawn(NewSession::new(shell_spawn()))
        .expect("spawn 1");
    registry
        .spawn(NewSession::new(shell_spawn()))
        .expect("spawn 2");
    let err = registry
        .spawn(NewSession::new(shell_spawn()))
        .expect_err("cap");
    assert!(err.to_string().contains("session limit"));
    // The rejected spawn created nothing: still exactly 2 live.
    assert_eq!(registry.len(), 2);
    registry.remove(&first).expect("remove");
    // A freed slot accepts a new session with the next id.
    let third = registry
        .spawn(NewSession::new(shell_spawn()))
        .expect("spawn 3");
    assert_eq!(third, "sess_003");
    registry.remove(&third).expect("remove");
    // One slot is still occupied by the second session; drain it.
    assert_eq!(registry.len(), 1);
    assert_eq!(registry.reap_idle(Duration::ZERO), 1);
    assert!(registry.is_empty());
}

#[test]
fn unknown_ids_name_themselves() {
    let mut registry = SessionRegistry::new();
    let err = registry.get_mut("sess_404").expect_err("lookup");
    assert!(err.to_string().contains("sess_404"));
    let err = registry.remove("sess_404").expect_err("remove");
    assert!(err.to_string().contains("sess_404"));
}

#[test]
fn idle_sessions_are_reaped() {
    let mut registry = SessionRegistry::new();
    let id = registry
        .spawn(NewSession::new(shell_spawn()))
        .expect("spawn");
    std::thread::sleep(Duration::from_millis(120));
    let reaped = registry.reap_idle(Duration::from_millis(50));
    assert_eq!(reaped, 1);
    assert!(registry.get_mut(&id).is_err(), "reaped session is gone");
    assert!(registry.is_empty());
}

#[test]
fn fresh_sessions_survive_reaping() {
    let mut registry = SessionRegistry::new();
    let id = registry
        .spawn(NewSession::new(shell_spawn()))
        .expect("spawn");
    let reaped = registry.reap_idle(Duration::from_secs(60));
    assert_eq!(reaped, 0);
    assert!(registry.get_mut(&id).is_ok());
    registry.remove(&id).expect("remove");
}
