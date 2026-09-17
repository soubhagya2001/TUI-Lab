//! Full condition matrix against canned views (docs/06 §6.1).

use tui_lab_assertions::{evaluate, Condition, ScreenView};

fn live() -> ScreenView {
    ScreenView::live("Welcome to Dashboard\n> Projects\n  Settings", (2, 5), true)
}

#[test]
fn text_conditions() {
    let view = live();
    assert!(evaluate(&Condition::TextVisible("Dashboard".into()), &view).passed);
    assert!(!evaluate(&Condition::TextVisible("Missing".into()), &view).passed);
    assert!(evaluate(&Condition::TextNotVisible("Unhandled panic".into()), &view).passed);
    assert!(!evaluate(&Condition::TextNotVisible("Dashboard".into()), &view).passed);
    assert!(evaluate(&Condition::TextRegex(r"Proj\w+".into()), &view).passed);
    assert!(!evaluate(&Condition::TextRegex(r"Z{9}".into()), &view).passed);
    // Invalid regex is a failed verdict, never a panic.
    assert!(!evaluate(&Condition::TextRegex("([".into()), &view).passed);
}

#[test]
fn exact_text_trims_edges() {
    let view = ScreenView::live("  Dashboard\n", (0, 0), false);
    assert!(evaluate(&Condition::ExactText("Dashboard".into()), &view).passed);
    assert!(!evaluate(&Condition::ExactText("Other".into()), &view).passed);
}

#[test]
fn cursor_and_change_conditions() {
    let view = live();
    assert!(evaluate(&Condition::CursorPosition { row: 2, col: 5 }, &view).passed);
    assert!(!evaluate(&Condition::CursorPosition { row: 3, col: 10 }, &view).passed);
    assert!(evaluate(&Condition::ScreenChanged(true), &view).passed);
    assert!(!evaluate(&Condition::ScreenChanged(false), &view).passed);
}

#[test]
fn process_conditions() {
    let ended = ScreenView {
        exit_code: Some(0),
        crashed: false,
        ..live()
    };
    assert!(evaluate(&Condition::ExitCode(0), &ended).passed);
    assert!(!evaluate(&Condition::ExitCode(1), &ended).passed);
    assert!(evaluate(&Condition::NotCrashed, &ended).passed);

    let crashed = ScreenView {
        crashed: true,
        ..live()
    };
    assert!(!evaluate(&Condition::NotCrashed, &crashed).passed);

    // Still running: exit-code assertions fail with a clear reason.
    let running = live();
    let verdict = evaluate(&Condition::ExitCode(0), &running);
    assert!(!verdict.passed);
    assert!(verdict.detail.contains("still running"));
}
