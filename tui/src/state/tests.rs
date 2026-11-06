//! The state machine, key press by key press: every rule the TUI has, with
//! no terminal and no pty — `press` is the only input and the state is the only
//! output, except where a save is what a press is supposed to produce.

use aricma_todo_lib::doc::Doc;
use ratatui::crossterm::event::{KeyCode, KeyModifiers};

use crate::state::action::{Action, Pane};
use crate::state::confirm::Confirm;
use crate::state::prompt::{Mode, Prompt};
use crate::testing::{app, press, press_with, save_text, screen, single, type_text};

#[test]
fn j_and_k_move_and_clamp_inside_the_focused_pane() {
    let (_tmp, mut app) = app(&[(
        "TODOs.md",
        "- [ ] a
- [ ] b
- [ ] c
",
    )]);
    app.pane = Pane::Todos;
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Down);
    assert_eq!(app.task_idx, 2);
    press(&mut app, KeyCode::Down);
    assert_eq!(app.task_idx, 2, "clamped at the last task");
    press(&mut app, KeyCode::Char('G'));
    press(&mut app, KeyCode::Up);
    assert_eq!(app.task_idx, 1);
}

#[test]
fn the_file_view_is_never_a_section() {
    let (_tmp, mut app) = app(&[("TODOs.md", "# one\n\n- [ ] a\n\n## two\n\n- [ ] b\n")]);
    for _ in 0..6 {
        press(&mut app, KeyCode::Tab);
        assert_ne!(
            app.pane,
            Pane::Todos,
            "a folder session has no todos section"
        );
    }
}

#[test]
fn the_outline_is_always_a_section_even_with_no_headings() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
    // an empty outline is where a heading gets made, so it is never hidden
    assert!(app.headings().is_empty());
    assert_eq!(app.sections(), [Pane::Files, Pane::Outline, Pane::History]);
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.pane, Pane::Outline);

    // and `n` there writes the first heading of the file
    press(&mut app, KeyCode::Char('n'));
    assert_eq!(app.mode, Mode::AddingHeading(Prompt::new("")));
    type_text(&mut app, "later");
    let action = press(&mut app, KeyCode::Enter);
    assert_eq!(save_text(action), "- [ ] a\n# later\n");
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(app.headings().len(), 1, "the outline has it now");
    assert_eq!(app.history[0].label, "heading · later");
}

#[test]
fn the_root_is_the_real_tree_not_the_dot_that_was_typed() {
    let (tmp, app) = app(&[("a/TODOs.md", "- [ ] a\n")]);
    // `app` opens the resolved path, which is what the header shows: `.` and `..`
    // are fine to type and useless to read back
    assert!(app.root.is_absolute(), "{}", app.root.display());
    assert_eq!(app.root, tmp.path().canonicalize().unwrap());
}

#[test]
fn a_session_on_a_file_says_what_it_cannot_read() {
    // a code file with no `TODO:` comment has nothing to show, so naming it is an
    // error rather than a session with an empty panel
    let scratch = tempfile::tempdir().unwrap();
    let code = scratch.path().join("main.rs");
    std::fs::write(&code, "fn main() {}\n").unwrap();
    let Err(error) = crate::state::App::open(&code) else {
        panic!("a file with no todos opened a session");
    };
    assert!(
        error.to_string().contains("neither a markdown todo file"),
        "{error}"
    );

    // the same file with one comment is a session
    std::fs::write(&code, "fn main() {} // TODO: check this\n").unwrap();
    let (_tmp, app) = (scratch, crate::state::App::open(&code).expect("a comment todo"));
    assert!(app.single);
    assert_eq!(app.tasks().len(), 1);

    // and a markdown file with no todos is still a place to write one
    let notes = std::env::temp_dir().join("aricma-empty-notes.md");
    std::fs::write(&notes, "# notes\n").unwrap();
    let app = crate::state::App::open(&notes).expect("an empty document");
    assert!(app.single);
    let _ = std::fs::remove_file(&notes);
}

#[test]
fn a_single_file_session_starts_on_its_todos_and_has_no_file_list() {
    let (_tmp, mut app) = single("- [ ] a\n");
    assert_eq!(app.pane, Pane::Todos, "a file is opened to work on its todos");
    // the same window as a folder session with the file panel taken out: the sections
    // down the left, the todos on the right
    assert_eq!(app.sections(), [Pane::Outline, Pane::History]);
    assert!(app.drawn(Pane::Todos));
    assert!(!app.drawn(Pane::Files));

    // `tab` walks the sidebar, and from the right-hand panel it does nothing — the same
    // as `tab` in the file view, where `←` is the way back
    press(&mut app, KeyCode::Char('2'));
    assert_eq!(app.pane, Pane::Outline);
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.pane, Pane::History);
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.pane, Pane::Outline, "tab loops the sidebar");
    // and `0` is always the right-hand panel, which `←` leaves like any other
    press(&mut app, KeyCode::Char('0'));
    assert_eq!(app.pane, Pane::Todos);
    press(&mut app, KeyCode::Left);
    assert_eq!(app.pane, Pane::Outline, "`←` back to the sidebar");
    // and `→` crosses back into it: the two arrows are one pair, in both layouts
    press(&mut app, KeyCode::Right);
    assert_eq!(app.pane, Pane::Todos, "`→` into the right-hand panel");
    press(&mut app, KeyCode::Left);
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.pane, Pane::History);
    press(&mut app, KeyCode::Right);
    assert_eq!(app.pane, Pane::Todos);
    press(&mut app, KeyCode::Left);
    assert_eq!(app.pane, Pane::History, "`←` returns to the section it left");
}

#[test]
fn a_number_goes_to_the_panel_that_carries_it() {
    let (_tmp, mut app) = app(&[("TODOs.md", "# one\n\n- [ ] a\n")]);
    assert_eq!(app.pane, Pane::Files);
    press(&mut app, KeyCode::Char('3'));
    assert_eq!(app.pane, Pane::History);
    press(&mut app, KeyCode::Char('2'));
    assert_eq!(app.pane, Pane::Outline);
    press(&mut app, KeyCode::Char('0'));
    assert_eq!(app.pane, Pane::View);
    press(&mut app, KeyCode::Char('1'));
    assert_eq!(app.pane, Pane::Files);

    // a panel that is not on screen is not a destination: a single file has no
    // file list, so `1` does nothing at all
    let (_tmp, mut alone) = single("- [ ] a\n");
    assert_eq!(alone.pane, Pane::Todos);
    press(&mut alone, KeyCode::Char('1'));
    assert_eq!(alone.pane, Pane::Todos);
    press(&mut alone, KeyCode::Char('0'));
    assert_eq!(alone.pane, Pane::Todos);
}

#[test]
fn a_collection_takes_the_number_of_the_panel_it_replaces() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a #home\n")]);
    press(&mut app, KeyCode::Char('3'));
    // the collection row is under the files cursor, so open it from there
    press(&mut app, KeyCode::Char('1'));
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Right);
    assert_eq!(app.pane, Pane::Collection);
    press(&mut app, KeyCode::Char('0'));
    assert_eq!(app.pane, Pane::Collection);
    // and the index is gone while one is open, so `2` leaves the cursor alone
    press(&mut app, KeyCode::Char('2'));
    assert_eq!(app.pane, Pane::Collection);
}

#[test]
fn tab_loops_the_sidebar_and_the_arrows_cross_to_the_view() {
    let (_tmp, mut app) = app(&[("TODOs.md", "# one\n\n- [ ] a\n\n## two\n\n- [ ] b\n")]);
    assert_eq!(app.sections(), [Pane::Files, Pane::Outline, Pane::History]);
    assert_eq!(app.pane, Pane::Files);

    press(&mut app, KeyCode::Tab);
    assert_eq!(app.pane, Pane::Outline);
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.pane, Pane::History);
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.pane, Pane::Files, "tab loops the sidebar");

    // the arrows cross the gap, and come back to where they left
    press(&mut app, KeyCode::Right);
    assert_eq!(app.pane, Pane::View);
    press(&mut app, KeyCode::Left);
    assert_eq!(app.pane, Pane::Files);

    press(&mut app, KeyCode::Tab); // outline
    press(&mut app, KeyCode::Right);
    assert_eq!(app.pane, Pane::View);
    press(&mut app, KeyCode::Left);
    assert_eq!(app.pane, Pane::Outline, "back to the section it left");

    // tab is sidebar navigation, so inside the view it does nothing at all:
    // the left arrow is the way out, and it goes where you came from
    press(&mut app, KeyCode::Right);
    assert_eq!(app.pane, Pane::View);
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.pane, Pane::View, "tab does not move out of the view");
    press(&mut app, KeyCode::BackTab);
    assert_eq!(app.pane, Pane::View, "nor backwards");
    assert_eq!(app.focus_target(true), None);
    assert_eq!(app.focus_target(false), None);
    press(&mut app, KeyCode::Left);
    assert_eq!(app.pane, Pane::Outline, "left returns to the outline");
}

#[test]
fn moving_the_todo_moves_the_outline_to_the_heading_above_it() {
    let (_tmp, mut app) = app(&[(
        "TODOs.md",
        "# inbox\n\n- [ ] a\n- [ ] b\n\n## later\n\n- [ ] c\n- [ ] d\n",
    )]);
    app.pane = Pane::View;
    assert_eq!(app.outline_idx, 0, "a is under the first heading");

    press(&mut app, KeyCode::Down); // b
    assert_eq!(app.current_task().unwrap().title, "b");
    assert_eq!(app.outline_idx, 0, "still the first section");

    press(&mut app, KeyCode::Down); // c
    assert_eq!(app.current_task().unwrap().title, "c");
    assert_eq!(app.outline_idx, 1, "the outline marks ## later now");

    press(&mut app, KeyCode::Up); // b
    assert_eq!(app.outline_idx, 0, "and back again");

    press(&mut app, KeyCode::Char('G')); // d
    assert_eq!(app.outline_idx, 1);
}

#[test]
fn a_todo_above_every_heading_marks_the_first_one() {
    let (_tmp, mut app) = app(&[(
        "TODOs.md",
        "- [ ] loose\n\n## a\n\n- [ ] b\n\n## c\n\n- [ ] d\n",
    )]);
    app.pane = Pane::View;
    assert_eq!(app.outline_idx, 0, "no heading above it, so the first one");
    press(&mut app, KeyCode::Down); // b, under ## a
    assert_eq!(app.outline_idx, 0);
    press(&mut app, KeyCode::Down); // d, under ## c
    assert_eq!(app.outline_idx, 1);
}

#[test]
fn moving_the_outline_hands_over_to_that_sections_first_todo() {
    let (_tmp, mut app) = app(&[(
        "TODOs.md",
        "# one\n\n- [ ] a\n\n## two\n\n- [ ] b\n- [ ] c\n",
    )]);
    app.pane = Pane::Outline;
    assert_eq!(app.current_task().unwrap().title, "a");
    press(&mut app, KeyCode::Down); // "## two"
    assert_eq!(app.current_task().unwrap().title, "b");

    // the view is where the todos are walked, and the outline handed it "b"
    press(&mut app, KeyCode::Right);
    assert_eq!(app.pane, Pane::View);
    assert_eq!(app.current_task().unwrap().title, "b");
    press(&mut app, KeyCode::Down);
    assert_eq!(app.current_task().unwrap().title, "c");
}

#[test]
fn a_heading_with_no_todos_hands_over_to_the_nearest_one_before() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n\n## empty\n\n## end\n")]);
    app.pane = Pane::Outline;
    press(&mut app, KeyCode::Char('G')); // "## end", nothing after it
    assert_eq!(app.outline_idx, 1);
    assert_eq!(app.current_task().unwrap().title, "a");
}

#[test]
fn the_outline_moves_between_headings() {
    let (_tmp, mut app) = app(&[("TODOs.md", "# one\n\n## two\n\n### three\n\n- [ ] a\n")]);
    app.pane = Pane::Outline;
    assert_eq!(app.outline_idx, 0);
    press(&mut app, KeyCode::Down);
    assert_eq!(app.outline_idx, 1);
    press(&mut app, KeyCode::Char('G'));
    assert_eq!(app.outline_idx, 2);
    press(&mut app, KeyCode::Down);
    assert_eq!(app.outline_idx, 2, "clamped at the last heading");
    press(&mut app, KeyCode::Char('g'));
    assert_eq!(app.outline_idx, 0);
}

#[test]
fn x_and_space_check_and_uncheck_a_task() {
    let (_tmp, mut app) = app(&[(
        "TODOs.md",
        "- [ ] (A) two  spaces #x @y  
",
    )]);
    app.pane = Pane::Todos;
    let text = save_text(press(&mut app, KeyCode::Char('x')));
    assert_eq!(
        text,
        "- [x] (A) two  spaces #x @y  
"
    );
    let text = save_text(press(&mut app, KeyCode::Char(' ')));
    assert_eq!(
        text,
        "- [ ] (A) two  spaces #x @y  
"
    );
}

#[test]
fn the_letter_a_todo_already_carries_clears_the_priority() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
    app.pane = Pane::View;

    assert_eq!(
        save_text(press(&mut app, KeyCode::Char('B'))),
        "- [ ] (B) a\n"
    );
    assert_eq!(app.history[0].label, "set prio B · a");

    // the same letter again removes it
    assert_eq!(save_text(press(&mut app, KeyCode::Char('B'))), "- [ ] a\n");
    assert_eq!(app.history[0].label, "clear prio · a");

    // and a different letter replaces it
    press(&mut app, KeyCode::Char('B'));
    assert_eq!(
        save_text(press(&mut app, KeyCode::Char('C'))),
        "- [ ] (C) a\n"
    );
    assert_eq!(app.history[0].label, "set prio C · a");

    // pressing C again clears, C again sets it back
    assert_eq!(save_text(press(&mut app, KeyCode::Char('C'))), "- [ ] a\n");
    assert_eq!(
        save_text(press(&mut app, KeyCode::Char('C'))),
        "- [ ] (C) a\n"
    );
}

#[test]
fn uppercase_a_to_d_set_a_priority_and_the_letter_again_clears_it() {
    let (_tmp, mut app) = app(&[(
        "TODOs.md",
        "- [ ] a #t
",
    )]);
    app.pane = Pane::Todos;
    assert_eq!(
        save_text(press(&mut app, KeyCode::Char('B'))),
        "- [ ] (B) a #t
"
    );
    // the letter the todo carries clears it, which is what `0` used to do before
    // `0` became the todos panel's number
    assert_eq!(
        save_text(press(&mut app, KeyCode::Char('B'))),
        "- [ ] a #t
"
    );
    // and a third press sets it again: the key toggles, it does not clear twice
    assert_eq!(
        save_text(press(&mut app, KeyCode::Char('B'))),
        "- [ ] (B) a #t
"
    );
}

#[test]
fn d_asks_then_deletes_on_a_capital_y_and_cancels_on_anything_else() {
    let (_tmp, mut app) = app(&[(
        "TODOs.md",
        "- [ ] a
- [ ] b
",
    )]);
    app.pane = Pane::Todos;
    // any key cancels: a lowercase y is just a key, not a half-decision
    for cancel in [
        KeyCode::Char('n'),
        KeyCode::Char('y'),
        KeyCode::Enter,
        KeyCode::Esc,
        KeyCode::Char('x'),
    ] {
        press(&mut app, KeyCode::Char('d'));
        assert_eq!(
            app.mode,
            Mode::Confirming(crate::state::confirm::Confirm::Todo),
            "{cancel:?}: the question is open"
        );
        assert_eq!(press(&mut app, cancel), None, "{cancel:?} only cancels");
        assert_eq!(app.mode, Mode::Normal, "{cancel:?} dismissed it");
        assert_eq!(
            app.doc.render().lines().count(),
            2,
            "{cancel:?} deleted nothing"
        );
    }

    press(&mut app, KeyCode::Char('d'));
    let text = save_text(press(&mut app, KeyCode::Char('Y')));
    assert_eq!(
        text,
        "- [ ] b
"
    );
    assert_eq!(app.mode, Mode::Normal);
}

#[test]
fn escape_cancels_a_delete() {
    let (_tmp, mut app) = app(&[(
        "TODOs.md", "- [ ] a
",
    )]);
    app.pane = Pane::Todos;
    press(&mut app, KeyCode::Char('d'));
    assert_eq!(press(&mut app, KeyCode::Esc), None);
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(
        app.doc.render(),
        "- [ ] a
"
    );
}

#[test]
fn n_opens_an_empty_prompt_and_enter_appends_what_was_typed() {
    let (_tmp, mut app) = app(&[(
        "TODOs.md", "- [ ] a
",
    )]);
    app.pane = Pane::Todos;
    press(&mut app, KeyCode::Char('n'));
    assert_eq!(
        app.mode,
        Mode::Adding(crate::state::prompt::Prompt::new(""))
    );
    type_text(&mut app, "ship it #x @y");
    let text = save_text(press(&mut app, KeyCode::Enter));
    assert_eq!(
        text,
        "- [ ] a
- [ ] ship it #x @y
"
    );
    assert_eq!(app.task_idx, 1);
}

#[test]
fn n_refuses_an_empty_body_and_writes_nothing() {
    let (_tmp, mut app) = app(&[(
        "TODOs.md", "- [ ] a
",
    )]);
    app.pane = Pane::Todos;
    press(&mut app, KeyCode::Char('n'));
    assert_eq!(press(&mut app, KeyCode::Enter), None);
    // the field stays open on the row it prepared, and that row is the only thing
    // that exists: nothing has been saved and nothing is in the history
    assert_eq!(app.mode, Mode::Adding(Prompt::new("")));
    assert_eq!(app.doc.render(), "- [ ] a\n- [ ]\n");
    assert!(app.history.is_empty());
    assert_eq!(press(&mut app, KeyCode::Esc), None);
    assert_eq!(app.doc.render(), "- [ ] a\n", "a cancelled row is not a line");
}

#[test]
fn e_prefills_the_body_and_writes_it_back_as_typed() {
    let (_tmp, mut app) = app(&[(
        "TODOs.md",
        "  * [x] (A) old #tag @who
",
    )]);
    app.pane = Pane::Todos;
    press(&mut app, KeyCode::Char('e'));
    assert_eq!(
        app.mode,
        Mode::Editing(crate::state::prompt::Prompt::new("(A) old #tag @who"))
    );
    for _ in 0..5 {
        press(&mut app, KeyCode::Backspace);
    }
    type_text(&mut app, " new #café");
    let text = save_text(press(&mut app, KeyCode::Enter));
    assert_eq!(
        text,
        "  * [x] (A) old #tag new #café
"
    );
}

#[test]
fn deleting_the_last_task_leaves_the_selection_valid() {
    let (_tmp, mut app) = app(&[(
        "TODOs.md",
        "- [ ] only
",
    )]);
    app.pane = Pane::Todos;
    press(&mut app, KeyCode::Char('d'));
    save_text(press(&mut app, KeyCode::Char('Y')));
    assert_eq!(app.task_idx, 0);
    assert!(app.current_task().is_none());
    assert_eq!(press(&mut app, KeyCode::Up), None);
    assert_eq!(press(&mut app, KeyCode::Char('x')), None);
}

#[test]
fn switching_files_resets_the_task_selection_and_keeps_history_per_file() {
    let (_tmp, mut app) = app(&[
        (
            "a/TODOs.md",
            "- [ ] a1
- [ ] a2
",
        ),
        (
            "b/TODOs.md",
            "- [ ] b1
",
        ),
    ]);
    app.pane = Pane::Todos;
    press(&mut app, KeyCode::Down);
    app.press(KeyCode::Char('x'));
    assert_eq!(app.history.len(), 1);

    app.pane = Pane::Files;
    press(&mut app, KeyCode::Down);
    assert_eq!(app.file_idx, 1);
    assert_eq!(app.task_idx, 0);
    assert!(app.file_history().is_empty());

    press(&mut app, KeyCode::Up);
    assert_eq!(app.file_idx, 0);
    assert_eq!(app.file_history().len(), 1);
}

#[test]
fn every_edit_appends_a_history_entry_newest_first() {
    let (_tmp, mut app) = app(&[(
        "TODOs.md", "- [ ] a
",
    )]);
    app.pane = Pane::Todos;
    app.press(KeyCode::Char('x'));
    app.press(KeyCode::Char('B'));
    app.press(KeyCode::Char('n'));
    type_text(&mut app, "new");
    app.press(KeyCode::Enter);

    let labels: Vec<&str> = app
        .history
        .iter()
        .map(|entry| entry.label.as_str())
        .collect();
    assert_eq!(labels.len(), 3);
    assert_eq!(labels[0], "new todo · new");
    assert_eq!(labels[1], "set prio B · a");
    assert_eq!(labels[2], "check todo · a");
    assert!(app.history.iter().all(|entry| entry.applied));
}

#[test]
fn a_deleted_todo_toggles_once_each_way_and_never_duplicates() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n- [ ] b\n")]);
    app.pane = Pane::View;
    press(&mut app, KeyCode::Char('d'));
    press(&mut app, KeyCode::Char('Y'));
    assert_eq!(app.doc.render(), "- [ ] b\n", "deleted");

    app.pane = Pane::History;
    app.history_idx = 0;
    press(&mut app, KeyCode::Char('x'));
    assert_eq!(app.doc.render(), "- [ ] a\n- [ ] b\n", "undone");
    assert!(!app.history[0].applied, "the row says it is undone");

    // and redoing is the delete again, not another copy of the line
    press(&mut app, KeyCode::Char('x'));
    assert_eq!(app.doc.render(), "- [ ] b\n", "redone");
    assert!(app.history[0].applied, "the row says it is applied");

    // a third press undoes it again, once
    press(&mut app, KeyCode::Char('x'));
    assert_eq!(app.doc.render(), "- [ ] a\n- [ ] b\n");
}

#[test]
fn an_added_todo_toggles_once_each_way_and_never_duplicates() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
    app.pane = Pane::View;
    press(&mut app, KeyCode::Char('n'));
    type_text(&mut app, "new");
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.doc.render(), "- [ ] a\n- [ ] new\n");

    app.pane = Pane::History;
    app.history_idx = 0;
    press(&mut app, KeyCode::Char('x'));
    assert_eq!(app.doc.render(), "- [ ] a\n", "undone");
    press(&mut app, KeyCode::Char('x'));
    assert_eq!(app.doc.render(), "- [ ] a\n- [ ] new\n", "redone once");
}

#[test]
fn x_on_a_history_entry_reverts_it_and_x_again_redoes_it() {
    let (_tmp, mut app) = app(&[(
        "TODOs.md", "- [ ] a
",
    )]);
    app.pane = Pane::Todos;
    app.press(KeyCode::Char('x'));
    app.pane = Pane::History;

    let text = save_text(app.press(KeyCode::Char('x')));
    assert_eq!(
        text,
        "- [ ] a
"
    );
    assert!(!app.history[0].applied);

    let text = save_text(app.press(KeyCode::Char('x')));
    assert_eq!(
        text,
        "- [x] a
"
    );
    assert!(app.history[0].applied);
}

#[test]
fn reverting_a_stale_entry_is_refused_with_a_message() {
    let (_tmp, mut app) = app(&[(
        "TODOs.md", "- [ ] a
",
    )]);
    app.pane = Pane::Todos;
    app.press(KeyCode::Char('x'));
    app.pane = Pane::History;

    app.doc = Doc::parse(
        "- [x] someone else
",
    );
    assert_eq!(app.press(KeyCode::Char('x')), None);
    assert!(app.history[0].applied);
    let message = app.history[0].message.clone().expect("the row says why");
    assert!(message.text.contains("changed since then"), "{message:?}");
}

#[test]
fn opt_down_swaps_a_todo_with_the_one_below_it_and_the_cursor_follows() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n- [ ] b\n- [ ] c\n")]);
    app.pane = Pane::View;
    assert!(press_with(&mut app, KeyCode::Down, KeyModifiers::ALT).is_some());
    assert_eq!(app.doc.render(), "- [ ] b\n- [ ] a\n- [ ] c\n");
    // the same todo is still selected, one line further down
    assert_eq!(app.current_task().unwrap().body, "a");
    assert_eq!(app.history[0].label, "move todo · a");

    // and the move is one write, undone as one
    app.pane = Pane::History;
    app.history_idx = 0;
    press(&mut app, KeyCode::Char('x'));
    assert_eq!(app.doc.render(), "- [ ] a\n- [ ] b\n- [ ] c\n");
    assert_eq!(app.current_task().unwrap().body, "a", "back where it was");
}

#[test]
fn shift_arrows_go_to_the_first_and_last_row_of_the_section() {
    let (_tmp, mut app) = app(&[
        ("a/TODOs.md", "- [ ] a\n"),
        ("b/TODOs.md", "- [ ] b\n"),
        ("c/TODOs.md", "- [ ] c\n"),
    ]);
    app.pane = Pane::Files;
    press_with(&mut app, KeyCode::Down, KeyModifiers::SHIFT);
    assert_eq!(
        app.selected_file().unwrap().path,
        app.root.join("c/TODOs.md")
    );
    press_with(&mut app, KeyCode::Up, KeyModifiers::SHIFT);
    assert_eq!(
        app.selected_file().unwrap().path,
        app.root.join("a/TODOs.md")
    );

    // and in the todos, the same: the last one, then the first
    app.pane = Pane::View;
    app.file_idx = 0;
    let _ = app.open_current();
    press_with(&mut app, KeyCode::Down, KeyModifiers::SHIFT);
    assert_eq!(app.task_idx, app.tasks().len() - 1);
    press_with(&mut app, KeyCode::Up, KeyModifiers::SHIFT);
    assert_eq!(app.task_idx, 0);
    assert_eq!(
        app.doc.render(),
        "- [ ] a\n",
        "a jump is not a move: nothing was reordered"
    );
}

#[test]
fn home_and_end_jump_like_g_and_g_because_a_terminal_may_drop_the_modifier() {
    let (_tmp, mut app) = app(&[("TODOs.md", "# h\n\n- [ ] a\n- [ ] b\n- [ ] c\n")]);
    app.pane = Pane::View;
    assert_eq!(press(&mut app, KeyCode::End), None);
    assert_eq!(app.task_idx, 2, "the last todo");
    assert_eq!(press(&mut app, KeyCode::Home), None);
    assert_eq!(app.task_idx, 0, "the first");
    assert_eq!(
        app.doc.render(),
        "# h\n\n- [ ] a\n- [ ] b\n- [ ] c\n",
        "a jump is not a move"
    );
}

#[test]
fn opt_arrows_step_the_sidebar_and_ctrl_arrows_move_the_todo() {
    let (_tmp, mut app) = app(&[("TODOs.md", "# h\n\n- [ ] a\n- [ ] b\n")]);
    app.pane = Pane::Files;
    // opt is the terminal's choice of word: any of them steps the sidebar
    for modifiers in [KeyModifiers::ALT, KeyModifiers::SUPER] {
        press_with(&mut app, KeyCode::Down, modifiers);
        assert_eq!(app.pane, Pane::Outline, "{modifiers:?}");
        press_with(&mut app, KeyCode::Up, modifiers);
        assert_eq!(app.pane, Pane::Files, "{modifiers:?}");
    }

    // ctrl is the one that moves the todo, wherever the keyboard is
    app.pane = Pane::View;
    assert!(press_with(&mut app, KeyCode::Down, KeyModifiers::CONTROL).is_some());
    assert_eq!(app.doc.render(), "# h\n\n- [ ] b\n- [ ] a\n");

    // and in the file view opt still moves, because there is no sidebar there
    app.pane = Pane::View;
    assert!(press_with(&mut app, KeyCode::Up, KeyModifiers::ALT).is_some());
    assert_eq!(app.doc.render(), "# h\n\n- [ ] a\n- [ ] b\n");
}

#[test]
fn j_and_k_move_the_todo_when_the_terminal_eats_the_modifier() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n- [ ] b\n")]);
    app.pane = Pane::View;
    press(&mut app, KeyCode::Char('J'));
    assert_eq!(app.doc.render(), "- [ ] b\n- [ ] a\n");
    assert_eq!(
        app.current_task().unwrap().body,
        "a",
        "the cursor followed it"
    );
    press(&mut app, KeyCode::Char('K'));
    assert_eq!(app.doc.render(), "- [ ] a\n- [ ] b\n");
}

#[test]
fn opt_down_crosses_a_heading_into_the_section_below() {
    let source = "## one\n- [ ] a\n## two\n- [ ] b\n";
    let (_tmp, mut app) = app(&[("TODOs.md", source)]);
    app.pane = Pane::View;
    assert!(press_with(&mut app, KeyCode::Down, KeyModifiers::ALT).is_some());
    // `a` is the first todo under `## two` now, and `b` has not been dragged up
    assert_eq!(app.doc.render(), "## one\n## two\n- [ ] a\n- [ ] b\n");
    assert_eq!(app.current_task().unwrap().body, "a", "the cursor followed it");
    assert_eq!(app.history.len(), 1, "one move, one entry");
}

#[test]
fn n_in_the_file_panel_opens_a_field_on_the_selected_folder() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] root\n"), ("web/TODOs.md", "- [ ] a\n")]);
    app.pane = Pane::Files;
    // the one under `web/`, wherever the sort put it
    app.file_idx = app
        .files
        .iter()
        .position(|entry| entry.path.ends_with("web/TODOs.md"))
        .unwrap();
    press(&mut app, KeyCode::Char('n'));
    let Mode::Creating(create) = &app.mode else {
        panic!("the field is open: {:?}", app.mode);
    };
    // the field's `./` and `/TODOs.md` are its own frame; this is what is typed
    assert_eq!(create.typed(), "web/");
    // and the folder that is already there cannot be made twice
    assert!(create.refusal.is_some(), "TODOs.md is already in ./web");
}

#[test]
fn creating_a_file_makes_the_folders_and_opens_it() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] root\n")]);

    app.pane = Pane::Files;
    press(&mut app, KeyCode::Char('n'));
    assert_eq!(
        app.mode,
        Mode::Creating(crate::state::create::Create::new(&app.root, "")),
        "the field starts empty at the root"
    );
    for character in "deep/er".chars() {
        press(&mut app, KeyCode::Char(character));
    }
    let Mode::Creating(create) = &app.mode else {
        panic!("the field is open");
    };
    assert_eq!(create.refusal, None, "typed {:?}", create.typed());
    assert_eq!(press(&mut app, KeyCode::Enter), None);
    assert_eq!(app.mode, Mode::Normal);

    let made = app.root.join("deep/er/TODOs.md");
    assert!(made.is_file(), "the file was made");
    assert_eq!(app.files.len(), 2);
    assert_eq!(app.current().path, made, "and it is the open file");
    assert_eq!(app.folders(&made), vec!["deep", "er"]);
}

#[test]
fn enter_on_a_file_that_is_already_there_writes_nothing() {
    let (tmp, mut app) = app(&[("TODOs.md", "- [ ] root\n")]);
    app.pane = Pane::Files;
    press(&mut app, KeyCode::Char('n'));
    for character in ".".chars() {
        press(&mut app, KeyCode::Char(character));
    }
    let Mode::Creating(create) = &app.mode else {
        panic!("the field is open");
    };
    assert!(create.refusal.is_some(), "TODOs.md is at the root");
    let before = std::fs::read_to_string(tmp.path().join("TODOs.md")).unwrap();
    assert_eq!(press(&mut app, KeyCode::Enter), None);
    assert!(
        matches!(app.mode, Mode::Creating(_)),
        "the field stays open"
    );
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("TODOs.md")).unwrap(),
        before,
        "nothing was overwritten"
    );
}

#[test]
fn n_is_new_a_file_when_files_are_chosen_and_a_todo_when_todos_are() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);

    app.pane = Pane::Files;
    press(&mut app, KeyCode::Char('n'));
    assert!(matches!(app.mode, Mode::Creating(_)), "{:?}", app.mode);
    press(&mut app, KeyCode::Esc);

    app.pane = Pane::View;
    press(&mut app, KeyCode::Char('n'));
    assert_eq!(app.mode, Mode::Adding(Prompt::new("")));
    press(&mut app, KeyCode::Esc);

    // in the outline it makes a heading, and in the history it makes nothing
    app.pane = Pane::Outline;
    press(&mut app, KeyCode::Char('n'));
    assert_eq!(app.mode, Mode::AddingHeading(Prompt::new("")));
    press(&mut app, KeyCode::Esc);
    app.pane = Pane::History;
    press(&mut app, KeyCode::Char('n'));
    assert_eq!(app.mode, Mode::Normal, "the history makes nothing");
}

#[test]
fn a_file_the_session_made_is_born_with_its_heading_and_deletes_after_asking() {
    let (tmp, mut app) = app(&[("b/TODOs.md", "- [ ] b\n")]);
    app.pane = Pane::Files;
    // an empty file only exists if this session made one: the scan lists files
    // that hold a todo
    press(&mut app, KeyCode::Char('n'));
    for character in "a".chars() {
        press(&mut app, KeyCode::Char(character));
    }
    press(&mut app, KeyCode::Enter);
    // the field started inside the folder of the selected file, so the new one
    // lands there
    let made = tmp.path().join("b/a/TODOs.md");
    assert!(made.is_file(), "the file exists");
    // it opens with its own title, which is what everybody wrote by hand anyway
    assert_eq!(
        std::fs::read_to_string(&made).unwrap(),
        "# a todos\n\n",
        "the heading names the folder"
    );

    // a file always has content now, so it always asks
    assert_eq!(press(&mut app, KeyCode::Char('d')), None);
    assert_eq!(app.mode, Mode::Confirming(Confirm::File), "it asks");
    assert!(made.exists(), "nothing is deleted yet");
    press(&mut app, KeyCode::Char('Y'));
    assert!(!made.exists(), "the file is gone");
    assert_eq!(app.files.len(), 1);
    assert_eq!(app.current().path, app.root.join("b/TODOs.md"));
    assert_eq!(app.doc.render(), "- [ ] b\n", "the neighbour is open");
}

#[test]
fn d_asks_before_deleting_a_file_with_content_and_only_a_capital_y_does_it() {
    let (tmp, mut app) = app(&[("a/TODOs.md", "- [ ] a\n"), ("b/TODOs.md", "- [ ] b\n")]);
    app.pane = Pane::Files;
    app.file_idx = app
        .files
        .iter()
        .position(|entry| entry.path.ends_with("a/TODOs.md"))
        .unwrap();
    let mine = tmp.path().join("a/TODOs.md");

    press(&mut app, KeyCode::Char('d'));
    assert_eq!(app.mode, Mode::Confirming(Confirm::File), "it asks first");
    let text = screen(&app).join("\n");
    // a modal, so it cannot be missed, and it names what it is about
    assert!(text.contains("╭ delete file"), "a framed question: {text}");
    assert!(text.contains("a/TODOs.md"), "{text}");
    assert!(text.contains("Y delete"), "the deliberate key: {text}");
    press(&mut app, KeyCode::Esc);

    // everything that is not the capital cancels, and the file stays
    for cancel in [
        KeyCode::Char('n'),
        KeyCode::Char('y'),
        KeyCode::Enter,
        KeyCode::Esc,
    ] {
        press(&mut app, KeyCode::Char('d'));
        assert_eq!(app.mode, Mode::Confirming(Confirm::File), "{cancel:?} asks");
        press(&mut app, cancel);
        assert_eq!(app.mode, Mode::Normal, "{cancel:?} cancels");
        assert!(mine.exists(), "{cancel:?} deleted nothing");
    }

    // and the capital does it
    press(&mut app, KeyCode::Char('d'));
    assert_eq!(press(&mut app, KeyCode::Char('Y')), None);
    assert!(!mine.exists(), "a capital Y deletes");
    assert_eq!(app.files.len(), 1);
}

#[test]
fn deleting_the_last_file_has_nothing_left_to_show() {
    let (tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
    app.pane = Pane::Files;
    press(&mut app, KeyCode::Char('d'));
    press(&mut app, KeyCode::Char('Y'));
    assert!(!tmp.path().join("TODOs.md").exists());
    assert!(app.files.is_empty());
    assert!(app.quit, "there is nothing left to be about");
}

#[test]
fn deleting_a_file_forgets_the_writes_recorded_against_it() {
    let (tmp, mut app) = app(&[("a/TODOs.md", "- [ ] a\n"), ("b/TODOs.md", "- [ ] b\n")]);
    app.pane = Pane::Files;
    app.file_idx = app
        .files
        .iter()
        .position(|entry| entry.path.ends_with("a/TODOs.md"))
        .unwrap();
    app.pane = Pane::View;
    press(&mut app, KeyCode::Char('x'));
    assert_eq!(app.history.len(), 1, "a check was recorded");

    app.pane = Pane::Files;
    press(&mut app, KeyCode::Char('d'));
    press(&mut app, KeyCode::Char('Y'));
    assert!(app.history.is_empty(), "its lines are gone");
    assert!(!tmp.path().join("a/TODOs.md").exists());
}

#[test]
fn ctrl_c_quits_from_anywhere_and_the_command_key_opens_the_search() {
    // ctrl+c is the one key that works in a text field, where the letters are
    // letters: a terminal app has no other way out
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
    app.pane = Pane::View;
    press(&mut app, KeyCode::Char('n'));
    type_text(&mut app, "half");
    assert_eq!(
        press_with(&mut app, KeyCode::Char('c'), KeyModifiers::CONTROL),
        Some(Action::Quit),
        "quits even mid-word"
    );

    // and ctrl+f opens the popover, one modal at a time
    let (_second, mut other) = crate::testing::app(&[("TODOs.md", "- [ ] a\n")]);
    press_with(&mut other, KeyCode::Char('f'), KeyModifiers::CONTROL);
    assert_eq!(other.mode, Mode::Searching);
    press(&mut other, KeyCode::Esc);
    assert_eq!(other.mode, Mode::Normal);
}

#[test]
fn one_modal_at_a_time() {
    // the search never opens on top of a field: the field would be stranded
    // behind it with no way back to its keys
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
    app.pane = Pane::View;
    press(&mut app, KeyCode::Char('n'));
    type_text(&mut app, "half");
    press_with(&mut app, KeyCode::Char('f'), KeyModifiers::CONTROL);
    assert_eq!(
        app.mode,
        Mode::Adding(Prompt::new("half")),
        "the search stayed shut"
    );
    assert_eq!(press(&mut app, KeyCode::Char('q')), None, "still typing");
    assert_eq!(app.mode, Mode::Adding(Prompt::new("halfq")));

    // and a field never opens on top of the search
    let (_second, mut app) = crate::testing::app(&[("TODOs.md", "- [ ] a\n")]);
    press_with(&mut app, KeyCode::Char('f'), KeyModifiers::CONTROL);
    assert!(app.modal());
    assert_eq!(app.mode, Mode::Searching);
    assert_eq!(app.press(KeyCode::Char('n')), None);
    assert_eq!(app.search.query.text(), "n", "n is a letter here");
    assert_eq!(app.press(KeyCode::Char('?')), None);
    assert_eq!(app.search.query.text(), "n?", "and so is ?");
    assert_eq!(app.mode, Mode::Searching);
}

#[test]
fn q_quits_and_never_means_close() {
    // while the keys page is open it quits the app and leaves the page alone
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
    press(&mut app, KeyCode::Char('?'));
    assert_eq!(press(&mut app, KeyCode::Char('q')), Some(Action::Quit));
    assert_eq!(app.mode, Mode::Help, "the page was not closed by q");

    // in a delete confirmation it cancels the delete by quitting
    let (_two, mut app) = crate::testing::app(&[("TODOs.md", "- [ ] a\n")]);
    app.pane = Pane::View;
    press(&mut app, KeyCode::Char('d'));
    assert_eq!(press(&mut app, KeyCode::Char('q')), Some(Action::Quit));
    assert_eq!(app.doc.render(), "- [ ] a\n", "nothing was deleted");

    // but where text is typed it is the letter q, or a todo could never
    // contain one
    let (_three, mut app) = crate::testing::app(&[("TODOs.md", "- [ ] a\n")]);
    app.pane = Pane::View;
    press(&mut app, KeyCode::Char('n'));
    assert_eq!(press(&mut app, KeyCode::Char('q')), None, "q was typed");
    assert_eq!(app.mode, Mode::Adding(Prompt::new("q")));

    app.press(KeyCode::Esc);
    app.open_search();
    assert_eq!(app.press(KeyCode::Char('q')), None);
    assert_eq!(app.search.query.text(), "q");
}

#[test]
fn a_move_that_cannot_happen_says_why() {
    // the heading above is the document's first line: there is nothing above it
    let (_tmp, mut app) = app(&[("TODOs.md", "# inbox\n\n- [ ] a\n")]);
    app.pane = Pane::View;
    assert_eq!(press_with(&mut app, KeyCode::Up, KeyModifiers::ALT), None);
    let text = screen(&app).join("\n");
    assert!(text.contains("nothing above it to swap"), "{text}");
    assert_eq!(app.doc.render(), "# inbox\n\n- [ ] a\n", "nothing moved");

    // and at the end of the file there is nothing to swap with
    let (_last, mut last) = crate::testing::app(&[("TODOs.md", "- [ ] only\n")]);
    app.pane = Pane::View;
    last.pane = Pane::View;
    press_with(&mut last, KeyCode::Down, KeyModifiers::ALT);
    let text = screen(&last).join("\n");
    // the note wraps in a sidebar, so the check is the part that stays whole
    assert!(text.contains("nothing below it to swap"), "{text}");
}

#[test]
fn a_single_file_session_says_why_a_move_did_nothing() {
    let (_tmp, mut app) = single("# inbox\n\n- [ ] a\n");
    app.pane = Pane::Todos;
    press_with(&mut app, KeyCode::Up, KeyModifiers::ALT);
    let text = screen(&app).join("\n");
    assert!(text.contains("nothing above it to swap"), "{text}");
}

#[test]
fn the_keys_page_shows_the_key_that_arrived() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
    press(&mut app, KeyCode::Char('?'));
    press_with(&mut app, KeyCode::Down, KeyModifiers::ALT);
    let text = screen(&app).join("\n");
    assert!(text.contains("last key: Down + ALT"), "{text}");
}

#[test]
fn the_keys_page_opens_on_question_mark_and_closes_on_escape() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
    app.pane = Pane::Files;
    press(&mut app, KeyCode::Char('?'));
    assert_eq!(app.mode, Mode::Help);
    let text = screen(&app).join("\n");
    assert!(text.contains("esc close"), "{text}");
    // three columns: the shared keys, the panels that choose where you are, and
    // the one that edits a todo
    for title in ["general", "files", "index", "history", "todos", "collection"] {
        assert!(text.contains(title), "{title} is missing: {text}");
    }
    // one row per spelling, so a key that a terminal may not deliver is still on the
    // page: the letter, and the modifier form of the same jump or move
    for piece in [
        "new file",
        "search",
        "opt+↑/↓",
        "K / J",
        "ctrl+↑/↓",
        "shift+↑/↓",
        "home/end",
    ] {
        assert!(text.contains(piece), "{piece} is missing: {text}");
    }

    // nothing else does anything while it is open
    assert_eq!(app.press(KeyCode::Char('x')), None);
    assert_eq!(app.doc.render(), "- [ ] a\n");

    press(&mut app, KeyCode::Esc);
    assert_eq!(app.mode, Mode::Normal);
}

#[test]
fn an_empty_row_waits_while_the_add_field_floats_over_it() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] first\n")]);
    app.pane = Pane::View;
    press(&mut app, KeyCode::Char('n'));
    // the empty row is in the file before anything is typed, and the cursor is on it,
    // so the person sees where the todo lands while the field asks what it says
    assert_eq!(app.doc.render(), "- [ ] first\n- [ ]\n");
    assert_eq!(app.current_task().unwrap().line, 1);

    type_text(&mut app, "ship #infra");
    let text = screen(&app).join("\n");
    // the field is the question …
    assert!(text.contains("╭ new todo"), "{text}");
    assert!(text.contains("ship #infra"), "{text}");
    // … and the row it is about is on screen, waiting, and saying that a title is on
    // its way: a bare `- [ ]` would read as a broken todo
    assert!(text.contains("2 - [ ] …"), "{text}");
    // the dots are the screen's, not the file's
    assert_eq!(app.doc.render(), "- [ ] first\n- [ ]\n");
    assert!(text.contains("esc cancel · enter add"), "{text}");
    assert!(!text.contains("↑/↓ nav"), "{text}");
    // esc takes the row out and puts the cursor back on the row it came from
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.doc.render(), "- [ ] first\n");
    assert_eq!(app.current_task().unwrap().body, "first");

    // and enter writes the todo into the row that was waiting for it
    press(&mut app, KeyCode::Char('n'));
    type_text(&mut app, "ship #infra");
    let action = press(&mut app, KeyCode::Enter);
    let text = save_text(action);
    assert_eq!(text, "- [ ] first\n- [ ] ship #infra\n");
    assert_eq!(app.current_task().unwrap().body, "ship #infra");
    assert_eq!(app.history.len(), 1, "one entry: the line appearing");
}

#[test]
fn a_todo_with_no_title_is_refused_and_the_field_keeps_the_text() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] first\n")]);
    app.pane = Pane::View;
    press(&mut app, KeyCode::Char('n'));
    type_text(&mut app, "  #tag @who");
    assert_eq!(press(&mut app, KeyCode::Enter), None, "nothing was added");
    assert_eq!(
        app.doc.render(),
        "- [ ] first\n- [ ]\n",
        "the row waits, with no title on it"
    );
    assert_eq!(app.mode, Mode::Adding(Prompt::new("  #tag @who")));
    let text = screen(&app).join("\n");
    assert!(text.contains("! a todo needs a title"), "{text}");
}

#[test]
fn an_external_write_is_picked_up_without_leaving_a_note() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("TODOs.md");
    std::fs::write(&path, "- [ ] a\n").unwrap();
    let mut app = crate::state::App::open(&path).unwrap();

    std::fs::write(&path, "- [ ] a\n- [ ] written elsewhere\n").unwrap();
    app.refresh_from_disk().unwrap();

    assert_eq!(app.doc.render(), "- [ ] a\n- [ ] written elsewhere\n");
    // the screen is the notice: nothing is carried around to say so
    assert_eq!(app.message, None);
}

#[test]
fn our_own_save_is_not_an_external_write() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("TODOs.md");
    std::fs::write(&path, "- [ ] a\n").unwrap();
    let mut app = crate::state::App::open(&path).unwrap();
    app.pane = Pane::View;

    let text = save_text(app.press(KeyCode::Char('x')));
    let saved = aricma_todo_lib::store::save(&app.baseline, &text).unwrap();
    app.saved(saved);

    app.refresh_from_disk().unwrap();
    assert_eq!(app.doc.render(), "- [x] a\n", "the edit stands");
    assert!(app.history.len() == 1, "and its entry is still there");
}

#[test]
fn a_refresh_waits_while_something_is_being_typed() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("TODOs.md");
    std::fs::write(&path, "- [ ] a\n").unwrap();
    let mut app = crate::state::App::open(&path).unwrap();
    app.pane = Pane::View;
    app.press(KeyCode::Char('n'));
    type_text(&mut app, "half typed");

    std::fs::write(&path, "- [ ] a\n- [ ] elsewhere\n").unwrap();
    app.refresh_from_disk().unwrap();

    assert_eq!(
        app.doc.render(),
        "- [ ] a\n- [ ]\n",
        "the document is left alone, row and all"
    );
    assert_eq!(
        app.mode,
        Mode::Adding(crate::state::prompt::Prompt::new("half typed")),
        "and the typing survives"
    );
}

#[test]
fn reload_forgets_the_history() {
    let (_tmp, mut app) = app(&[(
        "TODOs.md", "- [ ] a
",
    )]);
    app.pane = Pane::Todos;
    app.press(KeyCode::Char('x'));
    assert_eq!(app.history.len(), 1);
    assert_eq!(app.press(KeyCode::Char('r')), None, "r is not a key");
    app.reload().unwrap();
    assert!(app.history.is_empty());
    assert_eq!(
        app.doc.render(),
        "- [ ] a
"
    );
}

#[test]
fn d_on_a_history_entry_reverts_it_and_forgets_it() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
    app.pane = Pane::View;
    press(&mut app, KeyCode::Char('x')); // check, which records one entry
    assert_eq!(app.history.len(), 1);

    app.pane = Pane::History;
    let text = save_text(press(&mut app, KeyCode::Char('d')));
    assert_eq!(text, "- [ ] a\n", "the write is undone");
    assert!(app.history.is_empty(), "and the entry is gone");
    assert_eq!(app.mode, Mode::Normal, "no confirmation was asked");
}

#[test]
fn d_on_a_reverted_entry_forgets_it_without_redoing_it() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
    app.pane = Pane::View;
    press(&mut app, KeyCode::Char('x'));
    app.pane = Pane::History;
    press(&mut app, KeyCode::Char('x')); // uncheck the entry: file is back to [ ]
    assert!(!app.history[0].applied);

    assert_eq!(
        press(&mut app, KeyCode::Char('d')),
        None,
        "nothing to write"
    );
    assert!(app.history.is_empty());
    assert_eq!(app.doc.render(), "- [ ] a\n");
}

/// check, uncheck, check again: three writes to one line, newest first.
fn chain() -> (tempfile::TempDir, crate::state::App) {
    let (tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
    app.pane = Pane::View;
    for _ in 0..3 {
        press(&mut app, KeyCode::Char('x'));
    }
    app.pane = Pane::History;
    (tmp, app)
}

#[test]
fn undoing_a_write_undoes_the_writes_that_sit_on_it() {
    let (_tmp, mut app) = chain();
    assert_eq!(app.history.len(), 3);
    assert!(app.history.iter().all(|entry| entry.applied));
    assert_eq!(app.dependency(0), Some(1), "the uncheck undoes the check");
    assert_eq!(app.dependency(1), Some(2));
    assert_eq!(app.dependency(2), None, "the first write undoes nothing");

    // the oldest write: everything above it goes with it
    app.history_idx = 2;
    assert!(press(&mut app, KeyCode::Char('x')).is_some());
    assert_eq!(app.doc.render(), "- [ ] a\n", "back to the start");
    assert!(app.history.iter().all(|entry| !entry.applied));
}

#[test]
fn undoing_the_newest_write_leaves_its_parent_alone() {
    let (_tmp, mut app) = chain();
    app.history_idx = 0;
    assert!(press(&mut app, KeyCode::Char('x')).is_some());
    assert_eq!(app.doc.render(), "- [ ] a\n", "one step back");
    assert!(!app.history[0].applied);
    assert!(app.history[1].applied, "its parent is still in effect");
    assert!(app.history[2].applied);
}

#[test]
fn a_write_cannot_be_redone_before_the_one_under_it() {
    let (_tmp, mut app) = chain();
    app.history_idx = 2;
    press(&mut app, KeyCode::Char('x'));
    assert_eq!(app.doc.render(), "- [ ] a\n");

    // the newest write goes on top of the middle one, which is also undone
    app.history_idx = 0;
    let before = app.doc.render();
    assert_eq!(press(&mut app, KeyCode::Char('x')), None, "no write");
    assert_eq!(app.doc.render(), before);
    // the row says why by itself, without anything being stored
    assert_eq!(app.message, None, "nothing is carried around");
    let message = app.message_for(0).expect("the row says why");
    assert!(message.text.contains("below"), "{message:?}");
    // a rule being explained is a warning, not a failure
    assert_eq!(message.tone, crate::state::message::Tone::Warning);

    // bottom up, and it works
    app.history_idx = 2;
    assert!(press(&mut app, KeyCode::Char('x')).is_some());
    assert_eq!(app.doc.render(), "- [x] a\n");
    app.history_idx = 1;
    assert!(press(&mut app, KeyCode::Char('x')).is_some());
    assert_eq!(app.doc.render(), "- [ ] a\n");
    app.history_idx = 0;
    assert!(press(&mut app, KeyCode::Char('x')).is_some());
    assert_eq!(app.doc.render(), "- [x] a\n");
}

#[test]
fn d_on_a_parent_write_undoes_its_dependents_and_forgets_it() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
    app.pane = Pane::View;
    press(&mut app, KeyCode::Char('x'));
    press(&mut app, KeyCode::Char('x'));
    assert_eq!(app.doc.render(), "- [ ] a\n");

    app.pane = Pane::History;
    app.history_idx = 1;
    assert!(press(&mut app, KeyCode::Char('d')).is_some(), "both undone");
    assert_eq!(app.history.len(), 1, "only the forgotten one went");
    assert!(app.history[0].label.starts_with("uncheck"));
    assert!(!app.history[0].applied);
    assert_eq!(app.doc.render(), "- [ ] a\n");
}

/// check, priority B, uncheck, priority clear: two dependency chains that
/// cross, and the priority ones nowhere near the checkbox ones.
fn crossed() -> (tempfile::TempDir, crate::state::App) {
    let (tmp, mut app) = app(&[("TODOs.md", "- [ ] (A) write the README\n")]);
    app.pane = Pane::View;
    press(&mut app, KeyCode::Char('x'));
    press(&mut app, KeyCode::Char('B'));
    press(&mut app, KeyCode::Char('x'));
    // B twice: set, then clear
    press(&mut app, KeyCode::Char('B'));
    app.pane = Pane::History;
    (tmp, app)
}

#[test]
fn a_dependency_is_the_write_it_undoes_not_whatever_came_before() {
    let (_tmp, app) = crossed();
    assert_eq!(app.history.len(), 4);
    assert_eq!(app.history[0].label, "clear prio · write the README");
    assert_eq!(app.history[1].label, "uncheck todo · write the README");
    assert_eq!(app.history[2].label, "set prio B · write the README");
    assert_eq!(app.history[3].label, "check todo · write the README");

    assert_eq!(app.dependency(0), Some(2), "the priority it cleared");
    assert_eq!(app.dependency(1), Some(3), "the check it undid");
    assert_eq!(app.dependency(2), None, "the first priority write");
    assert_eq!(app.dependency(3), None, "the first write of all");
}

#[test]
fn a_dependency_survives_a_toggle() {
    let (_tmp, mut app) = crossed();
    assert_eq!(app.dependency(0), Some(2));
    assert_eq!(app.dependency(1), Some(3));

    app.history_idx = 3;
    press(&mut app, KeyCode::Char('x'));
    assert_eq!(app.dependency(0), Some(2), "the priority pair is untouched");
    assert_eq!(
        app.dependency(1),
        Some(3),
        "the checkbox pair still hangs together"
    );

    // and back up again
    press(&mut app, KeyCode::Char('x'));
    press(&mut app, KeyCode::Char('x'));
    assert_eq!(app.dependency(0), Some(2), "{:?}", app.history[0].forward);
    assert_eq!(app.dependency(1), Some(3), "{:?}", app.history[1].forward);
}

#[test]
fn undoing_a_write_leaves_the_writes_it_does_not_undo_applied() {
    let (_tmp, mut app) = crossed();
    assert_eq!(app.doc.render(), "- [ ] write the README\n");

    // undo the check: the uncheck hangs on it, the priority writes do not
    app.history_idx = 3;
    assert!(press(&mut app, KeyCode::Char('x')).is_some());
    assert_eq!(app.doc.render(), "- [ ] write the README\n");
    assert!(!app.history[3].applied, "the check is undone");
    assert!(!app.history[1].applied, "and the uncheck that hung on it");
    assert!(app.history[0].applied, "the priority clear stays");
    assert!(app.history[2].applied, "and the priority under it");

    // the newest priority write, undone on its own: only its own pair moves
    app.history_idx = 0;
    assert!(press(&mut app, KeyCode::Char('x')).is_some());
    assert_eq!(app.doc.render(), "- [ ] (B) write the README\n");
    assert!(!app.history[0].applied);
    assert!(app.history[2].applied, "the pair below it is untouched");
    assert!(!app.history[1].applied, "and so is the checkbox chain");
}

#[test]
fn d_on_a_stale_history_entry_keeps_it() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
    app.pane = Pane::View;
    press(&mut app, KeyCode::Char('x'));
    app.pane = Pane::History;
    app.doc = Doc::parse("- [x] written by someone else\n");

    assert_eq!(press(&mut app, KeyCode::Char('d')), None);
    assert_eq!(
        app.history.len(),
        1,
        "an applied write is not forgotten silently"
    );
    // the row says it, not the panel: the message belongs to the entry it is about
    let message = app.history[0].message.clone().expect("the row says why");
    assert!(message.text.contains("reloads on its own"), "{message:?}");
}

/// One OPS task, as `c` writes it and as another tool would hand it over.
const PASTED: &str = r#"[
  {"id": "x-1", "status": "open", "metadata": {"priority": "B", "tags": ["home"]},
   "title": "pay rent #home"},
  {"id": "x-2", "status": "done", "title": "already done"}
]"#;

#[test]
fn a_pasted_ops_document_lands_as_todo_lines_under_the_cursor() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] first\n- [ ] last\n")]);
    app.pane = Pane::View;
    // the cursor is on `first`, so the pasted todos belong under it
    app.on_paste(PASTED);
    assert_eq!(
        app.doc.render(),
        "- [ ] first\n- [ ] (B) pay rent #home\n- [x] already done\n- [ ] last\n"
    );
    // the file got lines, not the document
    assert!(!app.doc.render().contains('{'), "no json in the file");
    // one entry per line, so undo peels them from the bottom up
    assert_eq!(app.history.len(), 2);
    assert!(app.message_text().contains("pasted 2 todos"), "{:?}", app.message);
}

#[test]
fn a_paste_that_is_not_ops_writes_nothing() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] first\n")]);
    app.pane = Pane::View;
    app.on_paste("- [ ] copied from an editor\n");
    assert_eq!(app.doc.render(), "- [ ] first\n", "nothing was written");
    assert!(app.history.is_empty());
    assert!(
        app.message_text().contains("not an OPS document"),
        "{:?}",
        app.message
    );
}

#[test]
fn a_paste_into_a_field_is_text_and_a_paste_into_a_file_list_says_where() {
    // a field is a text field: the document goes in as text, at the cursor
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] first\n")]);
    app.pane = Pane::View;
    press(&mut app, KeyCode::Char('e'));
    app.on_paste("with spaces");
    assert_eq!(
        app.mode,
        Mode::Editing(crate::state::prompt::Prompt::new("firstwith spaces")),
        "pasted at the cursor"
    );

    // a file list is not a place a todo can land, and says so
    press(&mut app, KeyCode::Esc);
    app.pane = Pane::Files;
    app.on_paste(PASTED);
    assert!(app.message_text().contains("goes into a file's todos"), "{:?}", app.message);
    assert_eq!(app.doc.render(), "- [ ] first\n");
}

#[test]
fn c_copies_the_file_path_the_ops_document_or_the_collection_name() {
    let (_tmp, mut app) = app(&[("a/TODOs.md", "- [ ] (A) pay rent #home @sam\n")]);

    // a file gives its absolute path: that is what pastes into a shell
    app.pane = Pane::Files;
    let action = press(&mut app, KeyCode::Char('c')).expect("a copy");
    let Action::Copy { text } = action else {
        panic!("{action:?}");
    };
    assert_eq!(text, app.root.join("a/TODOs.md").display().to_string());
    assert!(
        app.message_text().contains("copied"),
        "it says what happened: {:?}",
        app.message
    );

    // a todo gives an OPS document: the todo as data, not as a line
    app.pane = Pane::View;
    let action = press(&mut app, KeyCode::Char('c')).expect("a copy");
    let Action::Copy { text } = action else {
        panic!("{action:?}");
    };
    for piece in [
        "\"title\": \"pay rent #home @sam\"",
        "\"status\": \"open\"",
        "\"priority\": \"A\"",
        "\"tags\"",
        "\"assignee\": \"sam\"",
    ] {
        assert!(text.contains(piece), "{piece} is missing: {text}");
    }
    assert!(text.trim_start().starts_with('{'), "a task, not a page: {text}");
    // the header says what was copied in the words of the line, not of the document
    assert!(
        app.message_text().contains("copied - [ ] (A) pay rent"),
        "{:?}",
        app.message
    );

    // and the history, which has no single line to give, does nothing
    // a collection row gives the word it is, not the open file's path
    app.pane = Pane::Files;
    app.row_idx = app.files.len();
    let action = press(&mut app, KeyCode::Char('c')).expect("a copy");
    let Action::Copy { text } = action else {
        panic!("{action:?}");
    };
    assert_eq!(text, "#home");
    assert!(
        app.message_text().contains("copied #home"),
        "{:?}",
        app.message
    );

    app.pane = Pane::History;
    assert_eq!(press(&mut app, KeyCode::Char('c')), None);
}

#[test]
fn a_collection_row_gathers_its_todos_and_edits_the_origin_file() {
    let (tmp, mut app) = app(&[
        ("a/TODOs.md", "# a\n\n- [ ] pay rent #home\n"),
        ("b/TODOs.md", "# b\n\n- [ ] buy milk #home\n"),
    ]);
    app.pane = Pane::Files;
    // collections are rows after the files, so the keys that walk files walk them
    let at = app
        .rows
        .iter()
        .position(|row| row.collection().is_some_and(|one| one.name == "home"))
        .expect("the tag is a row");
    assert_eq!(at, app.files.len(), "after the files");

    // moving onto the row is enough: the collection is drawn beside it at once,
    // while the keyboard is still the file panel's
    app.row_idx = at - 1;
    press(&mut app, KeyCode::Down);
    assert_eq!(app.row_idx, at);
    assert_eq!(app.pane, Pane::Files, "the keyboard has not moved");
    assert_eq!(app.viewing().unwrap().name, "home", "but the view has");
    assert!(app.current().path.ends_with("a/TODOs.md"));
    assert_eq!(
        app.selected_file().map(|file| file.path.clone()),
        None,
        "a collection row is not a file"
    );

    // and `→` only hands it the keyboard
    press(&mut app, KeyCode::Right);
    assert_eq!(app.pane, Pane::Collection);
    assert_eq!(app.viewing().unwrap().name, "home");
    // gathered in file order, both files, and the open document is the first one's
    assert_eq!(app.items().len(), 2);
    assert!(app.items()[0].path.ends_with("a/TODOs.md"));
    assert!(app.current().path.ends_with("a/TODOs.md"));

    // checking writes to the file that todo came from, and nowhere else
    let text = save_text(press(&mut app, KeyCode::Char('x')));
    assert_eq!(text, "# a\n\n- [x] pay rent #home\n");
    assert_eq!(app.collection_stats().done, 1, "the collection knows");

    // moving down follows the todo into its own file
    press(&mut app, KeyCode::Down);
    assert!(app.current().path.ends_with("b/TODOs.md"));
    let text = save_text(press(&mut app, KeyCode::Char('x')));
    assert_eq!(text, "# b\n\n- [x] buy milk #home\n");
    assert_eq!(app.collection_stats().done, 2, "the collection knows both");

    // the history panel is that file's history
    assert_eq!(app.file_history().len(), 1);
    app.pane = Pane::History;

    // `←` gives the keyboard back, and the collection stays: the row is still it
    app.pane = Pane::Collection;
    press(&mut app, KeyCode::Left);
    assert_eq!(app.pane, Pane::Files);
    assert!(app.viewing().is_some(), "still what the row is about");

    // stepping onto a file row shows that file again
    press(&mut app, KeyCode::Up);
    assert!(app.viewing().is_none());
    assert_eq!(app.items().len(), 0);
    assert!(app.current().path.ends_with("b/TODOs.md"));
    let _ = tmp;
}

#[test]
fn h_shows_every_markdown_file_and_hides_them_again() {
    let (tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
    std::fs::write(tmp.path().join("backlog.md"), "- [ ] b\n").unwrap();
    app.pane = Pane::Files;
    let listed = |app: &crate::state::App| -> Vec<String> {
        app.files
            .iter()
            .map(|file| {
                file.path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect()
    };
    assert_eq!(listed(&app), ["TODOs.md"], "the convention hides the rest");

    // `h`: the rest of the markdown in the tree, which is where an unconventioned
    // team keeps its todos
    press(&mut app, KeyCode::Char('h'));
    assert_eq!(listed(&app), ["backlog.md", "TODOs.md"]);
    assert!(app.show_all);

    // and back: the file that was open is still open, even though it is hidden now
    press(&mut app, KeyCode::Char('h'));
    assert!(!app.show_all);
    assert_eq!(listed(&app), ["TODOs.md"]);
}

#[test]
fn g_and_shift_down_reach_the_ends_of_the_panel_from_a_collection_row() {
    let (_tmp, mut app) = app(&[
        ("a/TODOs.md", "- [ ] a #home\n"),
        ("b/TODOs.md", "- [ ] b #home\n"),
    ]);
    app.pane = Pane::Files;
    // the last row is the tag, not a file
    press(&mut app, KeyCode::Char('G'));
    assert_eq!(app.row_idx, app.rows.len() - 1);
    assert!(app.selected_file().is_none(), "the cursor is on the tag");

    // and back: the first row of the panel, which is a file
    press(&mut app, KeyCode::Char('g'));
    assert_eq!(app.row_idx, 0);
    assert_eq!(app.selected_file().unwrap().path, app.root.join("a/TODOs.md"));
}

#[test]
fn editing_a_collected_todo_edits_the_line_in_the_file_it_lives_in() {
    let (_tmp, mut app) = app(&[
        ("a/TODOs.md", "- [ ] one #home\n"),
        ("b/TODOs.md", "- [ ] two #home\n"),
    ]);
    app.pane = Pane::Files;
    app.row_idx = app.files.len();
    press(&mut app, KeyCode::Right);
    // the second item lives in b, and editing it writes there
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Char('e'));
    assert!(matches!(app.mode, Mode::Editing(_)), "the row is editable");
    // the field starts with the body and the cursor at its end
    for _ in 0.."two #home".len() {
        press(&mut app, KeyCode::Backspace);
    }
    for character in "two! #home".chars() {
        press(&mut app, KeyCode::Char(character));
    }
    let action = press(&mut app, KeyCode::Enter);

    // `Save` carries no path: the caller writes it to the file the app has open,
    // and that is the one the todo came from
    assert_eq!(save_text(action), "- [ ] two! #home\n");
    assert_eq!(app.current().path, app.root.join("b/TODOs.md"));
    assert_eq!(app.items()[1].body, "two! #home", "the collection agrees");
    assert_eq!(
        std::fs::read_to_string(app.root.join("a/TODOs.md")).unwrap(),
        "- [ ] one #home\n",
        "the other file is untouched"
    );
}

#[test]
fn enter_in_a_collection_opens_the_file_the_todo_lives_in() {
    let (_tmp, mut app) = app(&[
        ("a/TODOs.md", "# alpha\n\n## later\n\n- [ ] one #home\n"),
        ("b/TODOs.md", "# beta\n\n- [ ] two #home\n"),
    ]);
    app.pane = Pane::Files;
    // a write in b, so the history has something of b's to show
    press(&mut app, KeyCode::Down);
    assert_eq!(app.current().path, app.root.join("b/TODOs.md"));
    press(&mut app, KeyCode::Char('?')); // nothing: `?` is the keys page
    press(&mut app, KeyCode::Esc);

    // read the collection, and pick the todo that lives in b
    press(&mut app, KeyCode::Char('G'));
    press(&mut app, KeyCode::Right);
    assert_eq!(app.pane, Pane::Collection);
    press(&mut app, KeyCode::Down);
    assert_eq!(app.items()[app.task_idx].path, app.root.join("b/TODOs.md"));

    press(&mut app, KeyCode::Enter);

    // the collection is let go, and the file it came from takes over
    assert_eq!(app.pane, Pane::View, "the file's view has the keyboard");
    assert!(app.viewing().is_none(), "the collection is closed");
    assert_eq!(app.current().path, app.root.join("b/TODOs.md"));
    assert_eq!(app.task_idx, 0, "the todo we came for is selected");
    assert_eq!(app.doc.render(), "# beta\n\n- [ ] two #home\n");
    // the file panel's cursor is on that file's row, not on the tag's
    assert_eq!(app.row_idx, 1, "the second file's row");
    assert!(app.selected_file().is_some());
    // the index is back, on the heading the todo sits under
    assert!(app.sections().contains(&Pane::Outline));
    assert_eq!(app.headings()[app.outline_idx].text, "beta");
    // and the history is that file's
    assert!(app.file_history().iter().all(|at| app.history[*at].path == app.root.join("b/TODOs.md")));
}

#[test]
fn h_is_the_file_panels_key_and_not_the_collections() {
    let (tmp, mut app) = app(&[("TODOs.md", "- [ ] a #home\n")]);
    std::fs::write(tmp.path().join("backlog.md"), "- [ ] b #home\n").unwrap();
    app.pane = Pane::Files;
    // on the collection's row, the keyboard is still the file panel's: the switch
    // is about the scan, so it works here, and the preview is gathered again
    press(&mut app, KeyCode::Down);
    assert!(app.selected_file().is_none(), "the cursor is on the tag");
    assert_eq!(app.items().len(), 1, "only the convention's file is read");
    press(&mut app, KeyCode::Char('h'));
    assert!(app.show_all);
    assert!(app.files.iter().any(|file| {
        file.path
            .file_name()
            .is_some_and(|name| name == "backlog.md")
    }));
    assert_eq!(app.items().len(), 2);
    press(&mut app, KeyCode::Char('h'));
    assert!(!app.show_all);
    assert_eq!(app.items().len(), 1);

    // and in the collection itself it does nothing: this panel is for reading
    // todos, and the key that widens the *scan* belongs to the panel that lists
    // files
    // `G` is the last row, which is the tag again after the rescan moved the rows
    press(&mut app, KeyCode::Char('G'));
    press(&mut app, KeyCode::Right);
    assert_eq!(app.pane, Pane::Collection);
    let before = app.files.len();
    press(&mut app, KeyCode::Char('h'));
    assert!(!app.show_all, "the switch did not move");
    assert_eq!(app.files.len(), before, "and nothing was rescanned");
    assert_eq!(app.pane, Pane::Collection);
}

#[test]
fn a_collection_row_in_the_file_panel_has_nothing_to_delete_and_nothing_to_make() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] one #home\n")]);
    app.pane = Pane::Files;
    // the row below the files: the tag
    app.row_idx = app.files.len();
    assert!(
        app.selected_file().is_none(),
        "the cursor is on a collection"
    );

    // `d` would have to mean "remove this word from every line that says it",
    // which this tool does not do — so it asks nothing
    assert_eq!(press(&mut app, KeyCode::Char('d')), None);
    assert_eq!(app.mode, Mode::Normal, "no question opens");
    assert_eq!(app.doc.render(), "- [ ] one #home\n", "and nothing was cut");

    // and `n` would have no folder to put the file in
    assert_eq!(press(&mut app, KeyCode::Char('n')), None);
    assert_eq!(app.mode, Mode::Normal, "no field opens");
}

#[test]
fn a_collection_refuses_what_a_view_cannot_do() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] one #home\n- [ ] two #home\n")]);
    app.pane = Pane::Files;
    app.row_idx = app.files.len();
    press(&mut app, KeyCode::Right);
    assert_eq!(app.pane, Pane::Collection);

    // no new todo: it would have no file to go in
    press(&mut app, KeyCode::Char('n'));
    assert_eq!(app.mode, Mode::Normal, "no field opens here");

    // and no moving: a collection's order is the files' order
    press(&mut app, KeyCode::Char('J'));
    assert!(app.message_text().contains("no order"), "{:?}", app.message);
    assert_eq!(app.items()[0].body, "one #home", "nothing moved");

    // deleting a todo still works, and takes it out of the collection
    press(&mut app, KeyCode::Char('d'));
    press(&mut app, KeyCode::Char('Y'));
    assert_eq!(app.items().len(), 1);
    assert_eq!(app.items()[0].body, "two #home");
}

#[test]
fn checking_happens_in_the_view_and_nowhere_else() {
    let (_tmp, mut app) = app(&[("TODOs.md", "# one\n\n- [ ] a\n\n## two\n\n- [ ] b\n")]);
    let source = app.doc.render();

    // the files section is for picking a file, not for checking
    assert_eq!(app.pane, Pane::Files);
    assert_eq!(press(&mut app, KeyCode::Char('x')), None);
    assert_eq!(press(&mut app, KeyCode::Char(' ')), None);
    assert_eq!(app.doc.render(), source);

    // and so is the outline
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.pane, Pane::Outline);
    assert_eq!(press(&mut app, KeyCode::Char('x')), None);
    assert_eq!(app.doc.render(), source);

    // the view is where a todo is checkable
    press(&mut app, KeyCode::Right);
    assert_eq!(app.pane, Pane::View);
    let text = save_text(press(&mut app, KeyCode::Char('x')));
    assert_eq!(text, "# one\n\n- [x] a\n\n## two\n\n- [ ] b\n");
}

#[test]
fn there_are_no_vim_motions() {
    let (_tmp, mut app) = app(&[("TODOs.md", "# one\n\n- [ ] a\n\n## two\n\n- [ ] b\n")]);
    for key in "jklh".chars().map(KeyCode::Char) {
        assert_eq!(app.press(key), None, "{key:?} must do nothing");
    }
    assert_eq!(app.pane, Pane::Files, "still on the first section");
    assert_eq!(app.current_task().unwrap().title, "a");

    // the arrows do the moving instead: down is "inside this panel", so the
    // todos are walked once the arrows have crossed into the view
    press(&mut app, KeyCode::Right);
    assert_eq!(app.pane, Pane::View);
    press(&mut app, KeyCode::Down);
    assert_eq!(app.current_task().unwrap().title, "b");
    press(&mut app, KeyCode::Up);
    assert_eq!(app.current_task().unwrap().title, "a");
}

#[test]
fn the_files_section_does_not_touch_a_todo_and_its_key_makes_a_file() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
    assert_eq!(app.pane, Pane::Files);
    assert_eq!(
        press(&mut app, KeyCode::Char('x')),
        None,
        "no checking here"
    );
    assert_eq!(app.doc.render(), "- [ ] a\n");

    // `n` here makes a file, not a todo: the panel chooses files, so that is
    // what its one key makes
    press(&mut app, KeyCode::Char('n'));
    assert!(matches!(app.mode, Mode::Creating(_)), "{:?}", app.mode);
    assert_eq!(app.doc.render(), "- [ ] a\n");
}

#[test]
fn q_quits_and_saves_carry_the_open_path() {
    let (tmp, mut app) = app(&[(
        "TODOs.md", "- [ ] a
",
    )]);
    app.pane = Pane::Todos;
    assert_eq!(
        app.press(KeyCode::Char('x')),
        Some(Action::Save {
            text: "- [x] a
"
            .to_owned()
        })
    );
    assert!(tmp.path().join("TODOs.md").exists());
    assert_eq!(app.press(KeyCode::Char('q')), Some(Action::Quit));
}

#[test]
fn a_comment_todo_file_opens_on_its_todo_comments() {
    let (_tmp, mut app) = app(&[
        (
            "src/main.rs",
            "fn main() {\n    // TODO: wire it up #infra\n}\n",
        ),
        ("TODOs.md", "- [ ] a document todo\n"),
    ]);
    // a document is what a session opens by default, however the paths sort
    assert!(app.current().path.ends_with("TODOs.md"));
    assert!(!app.comment());

    let first = app.files[0].path.clone();
    app.focus_file(&first);
    app.open_current().unwrap();
    assert!(app.comment(), "a code file is a comment todo file");
    let tasks = app.tasks();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].title, "wire it up");
    assert_eq!(tasks[0].line, 1);
    // the line in the file is the line on screen: nothing was rewritten
    assert_eq!(
        app.doc.render(),
        "fn main() {\n    // TODO: wire it up #infra\n}\n"
    );
    // and the tag it carries gathers it like any other todo
    assert!(app
        .files
        .iter()
        .any(|file| file.collections.iter().any(|one| one.name == "infra")));
}

#[test]
fn a_comment_todo_takes_only_the_keys_that_make_sense_for_it() {
    let (_tmp, mut app) = app(&[("src/a.rs", "// TODO: (B) fix it\nlet x = 1;\n")]);
    assert!(app.comment());
    app.pane = Pane::View;

    // the bar offers the three keys that work and none of the four that do not
    let bar = crate::ui::section_keys(&app);
    for offered in ["e edit", "A-D prio", "d delete", "c copy"] {
        assert!(bar.iter().any(|hint| hint == offered), "{offered} in {bar:?}");
    }
    for refused in ["x check", "n add", "K / J move", "space check"] {
        assert!(!bar.iter().any(|hint| hint.starts_with(refused)), "{refused} in {bar:?}");
    }

    // and the keys themselves say why instead of doing nothing
    press(&mut app, KeyCode::Char('x'));
    assert!(app.message_text().contains("closed by deleting it"), "{:?}", app.message_text());
    press(&mut app, KeyCode::Char('n'));
    assert!(!matches!(app.mode, Mode::Adding(_)));
    assert!(app.message_text().contains("written by hand"), "{:?}", app.message_text());
    press(&mut app, KeyCode::Char('K'));
    assert!(app.message_text().contains("stays with the code"), "{:?}", app.message_text());
    // the line never moved
    assert_eq!(app.doc.render(), "// TODO: (B) fix it\nlet x = 1;\n");
}

#[test]
fn editing_a_comment_todo_keeps_the_comment_and_writes_the_text_after_it() {
    let (_tmp, mut app) = app(&[("src/a.rs", "  // TODO: (B) fix it\n")]);
    app.pane = Pane::View;
    press(&mut app, KeyCode::Char('e'));
    for _ in 0.."fix it".len() {
        press(&mut app, KeyCode::Backspace);
    }
    type_text(&mut app, "ship it #x");
    let action = press(&mut app, KeyCode::Enter);
    let text = save_text(action);
    assert_eq!(text, "  // TODO: (B) ship it #x\n");
    // one history entry, and it undoes back to the line as it was
    assert_eq!(app.history.len(), 1);
}

#[test]
fn a_comment_todo_can_be_priced_and_deleted() {
    let (_tmp, mut app) = app(&[("src/a.rs", "// TODO: fix it\nlet x = 1;\n")]);
    app.pane = Pane::View;
    let action = press(&mut app, KeyCode::Char('A'));
    assert_eq!(save_text(action), "// TODO: (A) fix it\nlet x = 1;\n");

    press(&mut app, KeyCode::Char('d'));
    let action = press(&mut app, KeyCode::Char('Y'));
    assert_eq!(save_text(action), "let x = 1;\n");
}
