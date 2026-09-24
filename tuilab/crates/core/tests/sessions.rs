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
    registry.remove(&first, None).expect("remove");
    registry.remove(&second, None).expect("remove");
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
    registry.remove(&first, None).expect("remove");
    // A freed slot accepts a new session with the next id.
    let third = registry
        .spawn(NewSession::new(shell_spawn()))
        .expect("spawn 3");
    assert_eq!(third, "sess_003");
    registry.remove(&third, None).expect("remove");
    // One slot is still occupied by the second session; drain it.
    assert_eq!(registry.len(), 1);
    assert_eq!(registry.reap_idle(Duration::ZERO), 1);
    assert!(registry.is_empty());
}

#[test]
fn remove_with_quit_reaps_a_clean_exit() {
    // A shell that understands `exit`, so quit bytes end it gracefully.
    #[cfg(windows)]
    let (command, args, quit): (String, Vec<String>, &[u8]) =
        ("cmd".to_string(), vec!["/Q".to_string()], b"exit\r");
    #[cfg(not(windows))]
    let (command, args, quit): (String, Vec<String>, &[u8]) =
        ("sh".to_string(), Vec::new(), b"exit\n");
    let mut registry = SessionRegistry::new();
    let id = registry
        .spawn(NewSession::new(SpawnOptions {
            command,
            args,
            ..shell_spawn()
        }))
        .expect("spawn");
    // Let the shell finish booting before the quit bytes: under parallel
    // ConPTY load a brand-new console can miss early input entirely, which
    // looks identical to "quit bytes don't work".
    SessionRegistry::pump_once(
        registry.get_mut(&id).expect("session"),
        Duration::from_millis(300),
    );
    let closed = registry.remove(&id, Some(quit)).expect("remove");
    assert!(
        closed.exited_cleanly,
        "quit bytes must win the race vs kill"
    );
}

#[test]
fn unknown_ids_name_themselves() {
    let mut registry = SessionRegistry::new();
    let err = registry.get_mut("sess_404").expect_err("lookup");
    assert!(err.to_string().contains("sess_404"));
    let err = registry.remove("sess_404", None).expect_err("remove");
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
    registry.remove(&id, None).expect("remove");
}

/// R10: read-side polling is not usage — a long-held read loop must not
/// keep an abandoned session alive. The old pump stamp refreshed
/// `last_active` on every tick, so the reaper never fired for pollers.
#[test]
fn pump_once_does_not_refresh_activity() {
    let mut registry = SessionRegistry::new();
    let id = registry
        .spawn(NewSession::new(shell_spawn()))
        .expect("spawn");
    {
        // One client access, then a read loop far longer than the idle
        // window: pumps must leave the activity stamp alone.
        let session = registry.get_mut(&id).expect("session");
        for _ in 0..3 {
            SessionRegistry::pump_once(session, Duration::from_millis(30));
        }
    }
    let reaped = registry.reap_idle(Duration::from_millis(20));
    assert_eq!(reaped, 1, "reads must not refresh last_active");
    assert!(registry.is_empty());
}

/// Counterpart: a client that comes back after the idle window does keep
/// the session — writes, resizes, and in-flight waits all ride `get_mut`.
#[test]
fn client_access_refreshes_activity() {
    let mut registry = SessionRegistry::new();
    let id = registry
        .spawn(NewSession::new(shell_spawn()))
        .expect("spawn");
    std::thread::sleep(Duration::from_millis(120));
    registry.get_mut(&id).expect("client access");
    assert_eq!(
        registry.reap_idle(Duration::from_millis(50)),
        0,
        "a touched session is not idle"
    );
    // Reap (short kill grace) instead of `remove`, which would spend the
    // full close grace on a shell that never exits on its own.
    assert_eq!(registry.reap_idle(Duration::ZERO), 1);
    assert!(registry.is_empty());
}
