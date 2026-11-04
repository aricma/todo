//! What the scan's limits mean for the file list: a blocked file is stepped
//! over, never stopped on, and never opened.

use super::App;
use aricma_todo_lib::limits::MAX_LINES;

/// Only `todo*`, `task*` and `README.md` are candidates, and byte order puts
/// `README.md` before `TASKS.md` before `TODOs.md`.
fn folder(files: &[(&str, usize)]) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    for (name, lines) in files {
        let text: String = std::iter::repeat_n("- [ ] a line\n", *lines).collect();
        std::fs::write(tmp.path().join(name), text).unwrap();
    }
    tmp
}

fn shown(app: &App) -> String {
    app.current()
        .path
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned()
}

#[test]
fn a_blocked_file_is_stepped_over_not_stopped_on() {
    // README.md, TASKS.md (too long), TODOs.md: down from the first lands past
    let tmp = folder(&[
        ("README.md", 1),
        ("TASKS.md", MAX_LINES + 1),
        ("TODOs.md", 1),
    ]);
    let mut app = App::open(tmp.path()).unwrap();
    assert_eq!(shown(&app), "README.md");

    app.select_file(1).unwrap();
    assert_eq!(shown(&app), "TODOs.md", "down walked over the blocked file");

    app.select_file(-1).unwrap();
    assert_eq!(shown(&app), "README.md", "and up walks back over it");
}

#[test]
fn the_end_of_the_list_that_is_blocked_says_why() {
    let tmp = folder(&[("TASKS.md", 1), ("TODOs.md", MAX_LINES + 1)]);
    let mut app = App::open(tmp.path()).unwrap();
    app.select_file(1).unwrap();
    assert_eq!(shown(&app), "TASKS.md", "nowhere to go");
    assert!(
        app.message_text().contains("not opened"),
        "{:?}",
        app.message
    );

    // `G` on a list whose last file is blocked lands on the last openable one
    app.select_file(isize::MAX).unwrap();
    assert_eq!(shown(&app), "TASKS.md");
}

#[test]
fn a_folder_of_only_blocked_files_refuses_to_open() {
    let tmp = folder(&[("TODOs.md", MAX_LINES + 1)]);
    let error = match App::open(tmp.path()) {
        Ok(_) => panic!("a folder of blocked files should not open"),
        Err(error) => error.to_string(),
    };
    assert!(error.contains("more than"), "{error}");
    assert!(error.contains("not read"), "{error}");
}
