//! The terminal layer: what it promises the screen and the state.

use std::fs;

use ratatui::crossterm::event::KeyCode;

use super::perform::perform;
use crate::state::App;

#[test]
fn the_panic_hook_runs_and_keeps_the_hook_it_found() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let seen = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&seen);
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |_| flag.store(true, Ordering::SeqCst)));
    crate::terminal::hook::install();
    let caught = std::panic::catch_unwind(|| panic!("boom"));
    std::panic::set_hook(previous);
    assert!(caught.is_err(), "the panic was re-raised");
    assert!(
        seen.load(Ordering::SeqCst),
        "the hook it found still runs, so the message still gets printed"
    );
}

#[test]
fn a_save_that_hits_a_stale_file_keeps_the_edit_and_asks_which_version_wins() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("TODOs.md");
    fs::write(&path, "- [ ] a\n").unwrap();
    let mut app = App::open(&path).unwrap();
    app.press(KeyCode::Char('x')); // the single-file session starts on the index
    fs::write(&path, "- [ ] written by someone else\n").unwrap();

    // the edit is still in memory, and the question is open: only a person can say
    // which version is the one they meant
    let action = app.save_action();
    perform(&mut app, action);
    assert_eq!(app.doc.render(), "- [x] a\n");
    assert_eq!(app.mode, crate::state::prompt::Mode::Conflicted);

    // nothing dismisses it by accident
    app.press(KeyCode::Char('n'));
    assert_eq!(app.mode, crate::state::prompt::Mode::Conflicted);
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "- [ ] written by someone else\n"
    );

    // `t` takes the file's version, and the file is left alone
    app.press(KeyCode::Char('t'));
    assert_eq!(app.mode, crate::state::prompt::Mode::Normal);
    assert_eq!(app.doc.render(), "- [ ] written by someone else\n");
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "- [ ] written by someone else\n"
    );
}

#[test]
fn keeping_mine_writes_over_the_file_that_moved() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("TODOs.md");
    fs::write(&path, "- [ ] a\n").unwrap();
    let mut app = App::open(&path).unwrap();
    app.press(KeyCode::Char('x'));
    fs::write(&path, "- [ ] written by someone else\n").unwrap();

    let action = app.save_action();
    perform(&mut app, action);
    let action = app.press(KeyCode::Char('Y')).expect("an overwrite");
    perform(&mut app, action);
    assert_eq!(fs::read_to_string(&path).unwrap(), "- [x] a\n");
    assert_eq!(app.mode, crate::state::prompt::Mode::Normal);

    // and the baseline is the file we just wrote, so the next save is not stale
    let action = app.save_action();
    perform(&mut app, action);
    assert_eq!(app.mode, crate::state::prompt::Mode::Normal);
    assert_eq!(fs::read_to_string(&path).unwrap(), "- [x] a\n");
}
