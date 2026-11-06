//! What the screen says: every test here reads the rendered buffer, not the state.
//! Isolated in its own file because the drawing code it checks is short and these
//! are not.

use std::fmt::Write as _;

use ratatui::style::Modifier;

use crate::state::prompt::{Mode, Prompt};
use crate::testing::{
    app, press, press_with, row, screen, screen_at, single, terminal, type_text, HEIGHT, WIDTH,
};

fn texts(app: &crate::state::App) -> String {
    screen(app).join("\n")
}

#[test]
fn every_section_is_a_framed_section_with_its_title() {
    let (_tmp, app) = app(&[(
        "TODOs.md",
        "# inbox\n\n- [ ] a\n- [x] b\n- [x] c\n\n## later\n",
    )]);
    let lines = screen(&app);
    for title in ["files", "index", "history"] {
        assert!(
            lines.iter().any(|line| line.contains(title)),
            "{title} missing:\n{}",
            lines.join("\n")
        );
    }
    // the path appears twice: in the list row and in the view's frame title
    assert!(
        lines
            .iter()
            .any(|line| line.contains("TODOs.md") && line.contains("2/3")),
        "the list row carries the numbers:\n{}",
        lines.join("\n")
    );
}

#[test]
fn the_todos_panel_numbers_the_lines_it_shows() {
    let (_tmp, app) = single("# a\n\n- [ ] (A) pay rent\n- [x] ship it\n");
    let lines = screen(&app);
    let row = lines
        .iter()
        .position(|line| line.contains("(A) pay rent"))
        .expect("the todo is on screen");
    // the file's own line number, so a person can find the line again
    assert!(
        lines[row].contains("3 - [ ] (A) pay rent"),
        "{}",
        lines[row]
    );
    let next = lines
        .iter()
        .position(|line| line.contains("ship it"))
        .unwrap();
    assert!(lines[next].contains("4 - [x] ship it"), "{}", lines[next]);
}

#[test]
fn the_file_view_numbers_every_line_prose_included() {
    let (_tmp, app) = app(&[("TODOs.md", "# inbox\n\njust some prose\n\n- [ ] pay rent\n")]);
    let lines = screen(&app);
    let at = |needle: &str| {
        lines
            .iter()
            .position(|line| line.contains(needle))
            .unwrap_or_else(|| panic!("{needle} is on screen"))
    };
    // the same number the todos panel and the editor use, so a line found in one is
    // the line found in the other; prose and headings are numbered with the todos
    assert!(
        lines[at("just some prose")].contains("3 just some prose"),
        "{}",
        lines[at("just some prose")]
    );
    assert!(
        lines[at("pay rent")].contains("5 - [ ] pay rent"),
        "{}",
        lines[at("pay rent")]
    );
}

/// A screen against its recorded snapshot.
///
/// The header names the tree the session was opened on, and in a test that is a temp
/// directory — different every run. The path is masked, not dropped: the header is
/// part of the screen and has to be looked at too.
fn masked(app: &crate::state::App, screen: &str) -> String {
    // the header names the tree the session was opened on, and in a test that is a
    // fresh temp directory: its random name is masked, so the snapshot means
    // something. Only the name — a long path is cut from the front, and masking more
    // than the random part would depend on where the cut landed.
    let tree = if app.single {
        app.root.parent()
    } else {
        Some(app.root.as_path())
    };
    match tree.and_then(std::path::Path::file_name) {
        Some(name) => screen.replace(&*name.to_string_lossy(), "<tree>"),
        None => screen.to_owned(),
    }
}

/// The style of a `0/1` on a border, for a test that asks how it is drawn.
fn numbers_style(app: &crate::state::App) -> ratatui::style::Style {
    let lines = screen(app);
    // the view's own bottom border: the file list also says `0/1`, but that is a
    // row, not a border
    let at = lines
        .iter()
        .position(|line| line.contains("0/1") && line.contains('╯'))
        .expect("the numbers are on a border");
    let text = &lines[at];
    let column = u16::try_from(text[..text.find("0/1").unwrap()].chars().count()).unwrap();
    let buffer = crate::testing::terminal(app, WIDTH, HEIGHT)
        .backend()
        .buffer()
        .clone();
    let cell = &buffer[(column, u16::try_from(at).unwrap())];
    ratatui::style::Style::default()
        .fg(cell.fg)
        .add_modifier(cell.modifier)
}

#[test]
fn the_numbers_on_a_border_brighten_with_it() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
    // the panel has the cursor: its numbers are the border's own colour
    app.pane = crate::state::action::Pane::View;
    let focused = numbers_style(&app);
    assert_eq!(
        focused.fg,
        Some(super::consts::ACCENT),
        "the focused border's colour"
    );
    assert!(
        focused.add_modifier.contains(Modifier::BOLD),
        "and as bold as the border"
    );

    // and in a panel without the cursor they are dim, like its border
    app.pane = crate::state::action::Pane::History;
    let unfocused = numbers_style(&app);
    assert!(
        unfocused.add_modifier.contains(Modifier::DIM),
        "a panel without the cursor keeps its numbers quiet"
    );
}

#[test]
fn the_border_is_rounded_and_the_header_says_where_we_are() {
    let (_tmp, app) = single("- [ ] a\n");
    let lines = screen(&app);
    // the top line is the app and the tree it was opened on
    // one column of air, then the name; `✅` is two cells wide and the buffer
    // keeps the second one blank
    assert!(lines[0].starts_with(" ✅"), "{:?}", lines[0]);
    assert!(
        lines[0].trim_start().starts_with('✅'),
        "the header starts against the left edge, not on it: {:?}",
        lines[0]
    );
    assert!(
        lines[0].contains("aricma-lazytodo · "),
        "the app's name, then where it is: {:?}",
        lines[0]
    );
    assert!(
        lines[0].contains("TODOs.md"),
        "it names what was opened: {:?}",
        lines[0]
    );
    // the sections start under it, in the right-hand column: one file hides the
    // list, not the split
    assert!(lines[1].trim_start().starts_with('╭'), "{:?}", lines[1]);
    assert!(
        lines.iter().any(|line| line.trim_start().starts_with('╰')),
        "a bottom corner"
    );
}

#[test]
fn the_outline_shows_headings_and_no_todos() {
    // single-file mode has no raw file view, so a heading on screen can only
    // come from the outline
    let (_tmp, app) = single("# inbox\n\n- [ ] a todo the outline must not show\n\n## later\n");
    let text = texts(&app);
    assert!(text.contains("index"), "{text}");
    assert!(text.contains("inbox"), "{text}");
    assert!(text.contains("later"), "{text}");
    let outline_rows: Vec<&str> = text
        .lines()
        .skip_while(|line| !line.contains("index"))
        .take_while(|line| !line.contains("todos"))
        .collect();
    assert!(
        !outline_rows
            .iter()
            .any(|line| line.contains("must not show")),
        "{outline_rows:?}"
    );
}

#[test]
fn a_long_heading_wraps_under_its_own_text() {
    let (_tmp, app) = single(
        "# a heading long enough to need a second row in a narrow panel, easily\n\n- [ ] a\n",
    );
    // the index is only narrow when the whole screen is
    let lines = screen_at(&app, 40, 20);
    let first = lines
        .iter()
        .position(|line| line.contains("# a heading"))
        .expect("the heading is on screen");
    assert!(
        lines[first + 1].contains("enough to need a"),
        "the text continues on the row below: {:?}",
        &lines[first..first + 3]
    );
    // and the continuation lines up under the text, not under the hashes
    let text_at = lines[first].find("a heading").unwrap();
    let next_at = lines[first + 1].find("enough").unwrap();
    assert_eq!(text_at, next_at, "{:?}", &lines[first..first + 2]);
}

#[test]
fn the_outline_is_there_even_with_one_heading() {
    // an outline of one line says nothing, and it is still the place a second
    // heading gets made, so the panel stays
    let (_tmp, app) = app(&[("TODOs.md", "# only\n\n- [ ] a\n")]);
    let text = texts(&app);
    assert!(text.contains("index"), "{text}");
    assert!(text.contains("# only"), "{text}");
}

#[test]
fn a_folder_shows_the_file_pane_titled_with_the_path() {
    let (_tmp, app) = app(&[("a/TODOs.md", "- [ ] one\n"), ("b/TODOs.md", "- [ ] two\n")]);
    let lines = screen(&app);
    assert!(
        lines.iter().any(|line| line.contains("a/TODOs.md")),
        "{}",
        lines.join("\n")
    );
    assert!(row(&app, "one").is_some(), "the file's own text is shown");
}

#[test]
fn a_single_file_hides_the_list_and_the_file_view() {
    let (_tmp, app) = single("# a heading\n- [ ] one\n");
    let lines = screen(&app);
    assert!(
        !lines.iter().any(|line| line.contains("files")),
        "no file list"
    );
    // no panel is titled with the file, which is how the file view announces
    // itself; the heading still appears, in the outline
    assert!(
        !lines
            .iter()
            .any(|line| line.starts_with('╭') && line.contains("TODOs.md")),
        "no raw file view:\n{}",
        lines.join("\n")
    );
    assert!(
        lines.iter().any(|line| line.contains("todos")),
        "the todos list is the main section:\n{}",
        lines.join("\n")
    );
}

#[test]
fn todo_rows_show_marker_and_coloured_body() {
    // the second todo, because the selected row is flattened to the band's own
    // colours — the tags of the row the cursor is on are not what this checks
    let (_tmp, app) = single("- [ ] plain\n- [x] (A) pay rent #home @sam\n");
    let lines = screen(&app);
    let line = row(&app, "[x] (A) pay rent").expect("the row is on screen");
    let text = &lines[line];
    assert!(text.contains("#home @sam"), "{text}");

    let terminal = terminal(&app, WIDTH, HEIGHT);
    let buffer = terminal.backend().buffer();
    let cell = |needle: &str| {
        let column = u16::try_from(text.find(needle).unwrap()).unwrap();
        buffer[(column, u16::try_from(line).unwrap())].fg
    };
    assert_eq!(cell("#home"), super::prompt::TAG);
    assert_eq!(cell("@sam"), super::prompt::ASSIGNEE);
    assert_eq!(cell("(A)"), super::prompt::PRIO);
}

#[test]
fn editing_happens_on_the_todo_row_not_in_the_status_line() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] (A) pay rent #home @sam\n")]);
    app.pane = crate::state::action::Pane::Todos;
    app.press(ratatui::crossterm::event::KeyCode::Char('e'));
    for character in "now".chars() {
        app.press(ratatui::crossterm::event::KeyCode::Char(character));
    }
    let lines = screen(&app);
    let row = row(&app, "[ ] (A) pay rent #home @samnow").expect("the row shows what is typed");
    assert!(
        lines[row].contains("[ ] (A)"),
        "the checkbox stays: {}",
        lines[row]
    );
    assert!(
        !lines.join("\n").contains("edit:"),
        "nothing is typed into the status line:\n{}",
        lines.join("\n")
    );
}

#[test]
fn a_long_todo_wraps_and_prose_beside_it_does_not() {
    let long = "a todo that is far too long to fit in the panel once the \
                checkbox and the indent have taken their columns";
    let (wrapped, prose) = (format!("- [ ] {long}\n"), format!("{}\n", "p".repeat(400)));
    let (_tmp, app) = app(&[("TODOs.md", &format!("{prose}{wrapped}"))]);
    let lines = screen(&app);
    let first = row(&app, "a todo that is far too long").expect("the first row");
    assert!(
        lines[first].contains("a todo that is far too long"),
        "{}",
        lines[first]
    );
    assert!(
        lines[first + 1].contains("the panel once the checkbox"),
        "the tail is on the next row: {:?}",
        &lines[first..first + 3]
    );
    assert!(
        lines[first + 2].contains("columns"),
        "and finishes on a third: {:?}",
        &lines[first..first + 3]
    );
    // the continuation starts where the body starts: under the text, not under
    // the bullet — whatever the bullet and checkbox happen to be
    let body_at = lines[first].find("a todo that is far too long").unwrap();
    let body_column = lines[first][..body_at].chars().count();
    let below: Vec<char> = lines[first + 1].chars().collect();
    assert!(
        below[..body_column]
            .iter()
            .all(|character| *character == ' ' || *character == '│'),
        "nothing but border and air is left of the body: {}",
        lines[first + 1]
    );
    assert!(
        below[body_column] != ' ' && below[body_column] != '│',
        "the wrapped text itself starts there: {}",
        lines[first + 1]
    );
    // the prose above it is still one clipped row
    let prose_row = lines.iter().position(|line| line.contains("pppp")).unwrap();
    assert!(lines[prose_row].contains('…'), "{}", lines[prose_row]);
}

#[test]
fn a_copy_is_a_success_in_the_header_not_a_note_under_the_list() {
    let (_tmp, mut app) = app(&[
        ("a/TODOs.md", "- [ ] one #home\n"),
        ("b/TODOs.md", "- [ ] two\n"),
    ]);
    app.pane = crate::state::action::Pane::Files;
    press(&mut app, ratatui::crossterm::event::KeyCode::Char('c'));

    let lines = screen_at(&app, 96, 14);
    // the top line: where it is on the left, what worked on the right
    assert!(lines[0].contains("copied a/TODOs.md"), "{}", lines[0]);
    assert!(lines[0].contains("aricma-lazytodo"), "{}", lines[0]);
    // and nowhere else: the panel is unchanged by a message about nothing in it
    assert_eq!(
        lines.iter().filter(|line| line.contains("copied")).count(),
        1,
        "{}",
        lines.join("\n")
    );

    // green, which no other tone is
    let column = lines[0].find("copied").unwrap();
    let buffer = terminal(&app, 96, 14).backend().buffer().clone();
    let cell = &buffer[(u16::try_from(column).unwrap(), 0)];
    assert_eq!(cell.fg, super::consts::SUCCESS);
    assert!(!cell.modifier.contains(Modifier::DIM));
}

#[test]
fn a_rule_explained_stays_with_the_panel_it_is_about() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] one #home\n")]);
    // a collection has no order of its own: `J` is answered beside the list
    app.pane = crate::state::action::Pane::Files;
    app.row_idx = app.files.len();
    press(&mut app, ratatui::crossterm::event::KeyCode::Right);
    press(&mut app, ratatui::crossterm::event::KeyCode::Char('J'));

    let lines = screen_at(&app, 96, 16);
    assert!(
        lines.iter().any(|line| line.contains("no order")),
        "{}",
        lines.join("\n")
    );
    assert!(
        !lines[0].contains("no order"),
        "the header carries what worked, not what did not: {}",
        lines[0]
    );
}

#[test]
fn a_wrapped_todo_is_highlighted_whole() {
    let long = "a todo long enough that the panel has to wrap it onto a \
                second row so the highlight has somewhere to continue";
    let (_tmp, app) = single(&format!("- [ ] {long}\n- [x] short\n"));
    let terminal = terminal(&app, WIDTH, HEIGHT);
    let buffer = terminal.backend().buffer();
    let lines = screen(&app);

    let first = row(&app, "a todo long enough").expect("the first row");
    let second = first + 1;
    let column = u16::try_from(lines[first].find("a todo").unwrap()).unwrap();
    let selected =
        |row: usize| buffer[(column, u16::try_from(row).unwrap())].bg == super::consts::SELECTED;
    assert!(selected(first), "the first row is highlighted");
    assert!(
        selected(second),
        "so is the continuation: {:?}",
        &lines[first..=second]
    );

    // and the todo below it is not
    let next = row(&app, "[x] short").expect("the next todo");
    assert!(!selected(next), "only the selected todo is highlighted");
}

#[test]
fn prose_is_one_dim_truncated_line() {
    let long = format!("{} tail", "a".repeat(400));
    let (_tmp, app) = app(&[("TODOs.md", &format!("{long}\n- [ ] task\n"))]);
    let terminal = terminal(&app, WIDTH, HEIGHT);
    let buffer = terminal.backend().buffer();
    let lines = screen(&app);
    let prose_row = lines
        .iter()
        .position(|line| line.contains("aaaa"))
        .expect("prose is drawn");
    let column = u16::try_from(lines[prose_row].find("aaaa").unwrap()).unwrap();
    let cell = &buffer[(column, u16::try_from(prose_row).unwrap())];
    assert!(cell.modifier.contains(Modifier::DIM), "{cell:?}");
    assert!(lines[prose_row].contains('…'), "{}", lines[prose_row]);
    assert!(!lines[prose_row].contains("tail"), "{}", lines[prose_row]);
}

/// The rows of the files panel: the first framed column, without its borders.
fn files_panel(app: &crate::state::App) -> Vec<String> {
    let mut rows: Vec<String> = screen(app)
        .iter()
        .skip_while(|line| !line.contains("files"))
        .skip(1)
        .take_while(|line| !line.contains('╰'))
        .map(|line| {
            line.chars()
                .skip(2)
                .take_while(|character| *character != '│')
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        // the air around the selected row is not a row of the list
        .filter(|row| !row.is_empty())
        .collect();
    while rows.last().is_some_and(String::is_empty) {
        rows.pop();
    }
    rows
}

#[test]
fn the_active_panel_says_so_with_a_dot_and_not_only_a_colour() {
    let (_tmp, mut app) = app(&[("TODOs.md", "# one\n\n- [ ] a\n\n## two\n")]);
    let text = texts(&app);
    assert!(text.contains("╭ ● [1] files"), "{text}");
    assert_eq!(text.matches('●').count(), 1, "one dot, one cursor: {text}");

    // and the dot moves with the cursor
    press(&mut app, ratatui::crossterm::event::KeyCode::Tab);
    let text = texts(&app);
    assert!(text.contains("╭ ● [2] index"), "{text}");
    assert!(!text.contains("● [1] files"), "{text}");
}

#[test]
fn the_row_a_new_todo_waits_in_is_the_one_that_has_the_cursor() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
    app.pane = crate::state::action::Pane::View;
    press(&mut app, ratatui::crossterm::event::KeyCode::Char('n'));
    let lines = screen(&app);
    let at = lines
        .iter()
        .position(|line| line.contains("- [ ]"))
        .map(|first| first + 1)
        .expect("the empty row is on screen");
    // the empty todo is a line in the file, banded like the row it will be
    assert!(lines[at].contains("- [ ]"), "{:?}", &lines[..=at]);
    let buffer = terminal(&app, WIDTH, HEIGHT).backend().buffer().clone();
    let row = u16::try_from(at).unwrap();
    assert!(
        (0..WIDTH).any(|x| buffer[(x, row)].bg == super::consts::SELECTED),
        "the band, on the empty row: {:?}",
        lines[at]
    );

    // the field over it says so, and carries no dot: the row behind it is what has
    // the cursor
    let text = texts(&app);
    assert!(text.contains("╭ new todo"), "{text}");
    assert!(!text.contains("● new todo"), "{text}");
}

#[test]
fn a_modal_dims_everything_behind_it() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
    // the header's own text, bright while nothing is asked
    let column = u16::try_from(screen(&app)[0].find("aricma").unwrap()).unwrap();
    let cell = |app: &crate::state::App| {
        terminal(app, WIDTH, HEIGHT).backend().buffer()[(column, 0)].clone()
    };
    assert!(
        !cell(&app).modifier.contains(Modifier::DIM),
        "the screen is at full brightness"
    );

    app.pane = crate::state::action::Pane::View;
    press(&mut app, ratatui::crossterm::event::KeyCode::Char('n'));
    assert!(
        cell(&app).modifier.contains(Modifier::DIM),
        "the field has the keyboard, so the screen behind it is quiet"
    );

    // and the question itself is not dimmed with it
    let lines = screen(&app);
    let at = lines
        .iter()
        .position(|line| line.contains("new todo"))
        .expect("the field is on screen");
    let left = lines[at].find("╭ new todo").expect("a framed field");
    let edge = terminal(&app, WIDTH, HEIGHT).backend().buffer().clone();
    assert!(
        !edge[(u16::try_from(left).unwrap(), u16::try_from(at).unwrap())]
            .modifier
            .contains(Modifier::DIM),
        "the field's own border stays bright"
    );
}

#[test]
fn one_row_per_file_with_its_folders_in_front_of_it() {
    let (_tmp, app) = app(&[
        ("a/TODOs.md", "- [ ] a\n"),
        ("b/TODOs.md", "- [ ] b\n"),
        ("b/deep/Tasks.md", "- [ ] c\n"),
        ("README.md", "- [ ] d\n"),
    ]);
    // one row each, the folders named in it rather than drawn above it
    assert_eq!(
        files_panel(&app),
        vec![
            "a/TODOs.md  0/1",
            "b/deep/Tasks.md  0/1",
            "b/TODOs.md  0/1",
            "README.md  0/1",
        ]
    );
}

#[test]
fn every_selection_looks_the_same() {
    // a heading, a file and a todo are all "the row the cursor is on": one band of
    // colour, everywhere, so nothing has to be learned twice
    let (_tmp, app) = app(&[(
        "TODOs.md",
        "# first\n\n- [ ] a\n- [ ] b\n\n## second\n\n- [x] c\n",
    )]);
    let lines = screen(&app);
    let buffer = terminal(&app, WIDTH, HEIGHT).backend().buffer().clone();

    // the panels are side by side, so a row is found inside one of them
    let band = |title: &str, needle: &str| {
        let start = lines
            .iter()
            .position(|line| line.contains(title))
            .unwrap_or_else(|| panic!("no {title} panel:\n{}", lines.join("\n")));
        let (row, line) = lines
            .iter()
            .enumerate()
            .skip(start)
            .find(|(_, line)| line.contains(needle))
            .unwrap_or_else(|| panic!("{needle:?} is not in {title}:\n{}", lines.join("\n")));
        let at = line.find(needle).unwrap();
        let column = u16::try_from(line[..at].chars().count()).unwrap();
        buffer[(column, u16::try_from(row).unwrap())].bg
    };

    assert_eq!(
        band("files", "TODOs.md  1/3"),
        super::consts::SELECTED,
        "file"
    );
    // the outline's first heading is the one the cursor is on
    assert_eq!(band("index", "# first"), super::consts::SELECTED, "heading");
    assert_eq!(
        band("TODOs.md ─", "- [ ] a"),
        super::consts::SELECTED,
        "todo"
    );
}

#[test]
fn more_line_height_is_a_switch() {
    let (_tmp, mut app) = app(&[
        ("a/TODOs.md", "- [ ] a\n"),
        ("b/TODOs.md", "- [ ] b\n"),
        ("c/TODOs.md", "- [ ] c\n"),
    ]);
    app.pane = crate::state::action::Pane::Files;

    // dense by default: the selection is the band and nothing else, so the row
    // above and below it are the neighbours
    let lines = screen(&app);
    let at = lines
        .iter()
        .position(|line| line.contains("a/TODOs.md  0/1"))
        .expect("the row is on screen");
    assert!(lines[at + 1].contains("b/TODOs.md"), "{:?}", lines[at + 1]);

    // `t` gives every row a blank one under it: a terminal has no half-line, so
    // this is what "more line height" costs
    press(&mut app, ratatui::crossterm::event::KeyCode::Char('t'));
    assert!(app.roomy);
    let lines = screen(&app);
    let at = lines
        .iter()
        .position(|line| line.contains("a/TODOs.md  0/1"))
        .unwrap();
    assert!(lines[at + 1].starts_with("│   "), "{:?}", lines[at + 1]);
    assert!(lines[at + 2].contains("b/TODOs.md"), "{:?}", lines[at + 2]);

    // and back to dense
    press(&mut app, ratatui::crossterm::event::KeyCode::Char('t'));
    assert!(!app.roomy);
    let lines = screen(&app);
    let at = lines
        .iter()
        .position(|line| line.contains("a/TODOs.md  0/1"))
        .unwrap();
    assert!(lines[at + 1].contains("b/TODOs.md"), "{:?}", lines[at + 1]);
}

#[test]
fn a_long_history_entry_wraps_under_its_own_label() {
    let (_tmp, mut app) = app(&[(
        "TODOs.md",
        "- [ ] a todo with a title long enough to need two rows in this narrow sidebar\n",
    )]);
    app.pane = crate::state::action::Pane::View;
    press(&mut app, ratatui::crossterm::event::KeyCode::Char('x'));
    app.pane = crate::state::action::Pane::History;
    let lines = screen_at(&app, 80, 24);
    let text = lines.join("\n");
    let first = lines
        .iter()
        .position(|line| line.contains("check todo · a todo"))
        .unwrap_or_else(|| panic!("the entry is on screen:\n{text}"));
    assert!(
        lines[first + 1].contains("with a title"),
        "it wraps: {:?}",
        &lines[first..first + 3]
    );
    // the continuation lines up under the label, past the gutter and the dot. Columns,
    // not bytes: the dot is three bytes wide and one cell
    let column = |line: &str, needle: &str| line[..line.find(needle).unwrap()].chars().count();
    assert_eq!(
        column(&lines[first], "check todo"),
        column(&lines[first + 1], "with"),
        "{:?}",
        &lines[first..first + 2]
    );
}

#[test]
fn a_wrapped_history_entry_keeps_its_lane_running() {
    let (_tmp, mut app) = app(&[(
        "TODOs.md",
        "- [ ] a todo with a title long enough to need two rows in this narrow sidebar\n",
    )]);
    app.pane = crate::state::action::Pane::View;
    // two writes to the same line: the second one depends on the first, so a lane
    // runs from the newer entry down to the older one
    press(&mut app, ratatui::crossterm::event::KeyCode::Char('x'));
    press(&mut app, ratatui::crossterm::event::KeyCode::Char('x'));
    app.pane = crate::state::action::Pane::History;
    let lines = screen_at(&app, 80, 24);
    let first = lines
        .iter()
        .position(|line| line.contains("uncheck todo"))
        .unwrap_or_else(|| panic!("the entry is on screen:\n{}", lines.join("\n")));
    let lane = lines[first]
        .chars()
        .position(|one| one == '╭')
        .expect("the lane leaves this row");
    assert!(
        lines[first + 1].contains("with a title"),
        "it wraps: {:?}",
        &lines[first..first + 2]
    );
    // and the pipe beside the wrapped rows is still a pipe
    assert_eq!(
        lines[first + 1].chars().nth(lane),
        Some('│'),
        "{:?}",
        &lines[first..first + 3]
    );
}

#[test]
fn the_files_read_like_a_file_viewer_and_the_pointer_follows() {
    let (_tmp, mut app) = app(&[
        ("web/Tasks.md", "- [ ] w\n"),
        ("web/src/TODOs.md", "- [ ] s\n"),
        ("README.md", "- [ ] r\n"),
        ("crates/lib/TODOs.md", "- [ ] c\n"),
    ]);
    app.pane = crate::state::action::Pane::Files;
    // folders before files, everything compared without case, at every level
    assert_eq!(
        files_panel(&app),
        vec![
            "crates/lib/TODOs.md  0/1",
            "web/src/TODOs.md  0/1",
            "web/Tasks.md  0/1",
            "README.md  0/1",
        ]
    );

    // and `↓` walks them in exactly the order they are drawn
    let mut walked = Vec::new();
    for _ in 0..app.files.len() {
        walked.push(app.display(&app.current().path));
        press(&mut app, ratatui::crossterm::event::KeyCode::Down);
    }
    assert_eq!(
        walked,
        vec![
            "crates/lib/TODOs.md",
            "web/src/TODOs.md",
            "web/Tasks.md",
            "README.md",
        ]
    );
}

#[test]
fn a_fields_keys_are_on_the_bottom_border_esc_first_and_on_the_right() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
    app.pane = crate::state::action::Pane::View;
    press(&mut app, ratatui::crossterm::event::KeyCode::Char('n'));
    let lines = screen(&app);
    let border = lines
        .iter()
        .find(|line| line.contains("esc cancel"))
        .unwrap_or_else(|| panic!("{}", lines.join("\n")));
    assert!(border.contains("esc cancel · enter add"), "{border}");
    assert!(
        border.len() - border.trim_end().len() < 4 && border.find("esc cancel").unwrap() > 20,
        "right-aligned in the frame: {border}"
    );

    // and the same way round in the new-file field
    press(&mut app, ratatui::crossterm::event::KeyCode::Esc);
    app.pane = crate::state::action::Pane::Files;
    press(&mut app, ratatui::crossterm::event::KeyCode::Char('n'));
    let text = screen(&app).join("\n");
    assert!(text.contains("esc cancel · enter create"), "{text}");
}

#[test]
fn a_file_row_counts_its_tags_and_people_in_their_own_colours() {
    let (_tmp, mut app) = app(&[
        (
            "a/TODOs.md",
            "- [ ] one #home #work @sam @kim\n- [x] two #home\n",
        ),
        ("b/TODOs.md", "- [ ] three\n"),
    ]);
    // the colours belong to the rows that are not under the cursor's band
    app.pane = crate::state::action::Pane::Files;
    // the band follows the panel's cursor, so move that, not just the open file
    app.row_idx = 1;
    app.file_idx = 1;
    let lines = screen(&app);
    // the list row, not the view's title, which names the same file
    let row = lines
        .iter()
        .position(|line| line.contains("a/TODOs.md  1/2"))
        .expect("the file is listed");
    let text = &lines[row];
    // distinct, not per occurrence: two lines mention `#home`, it is one tag
    assert!(text.contains("#2"), "{text}");
    assert!(text.contains("@2"), "{text}");
    let column =
        |needle: &str| u16::try_from(text[..text.find(needle).unwrap()].chars().count()).unwrap();
    let buffer = terminal(&app, WIDTH, HEIGHT).backend().buffer().clone();
    let row = u16::try_from(row).unwrap();
    assert_eq!(buffer[(column("#2"), row)].fg, super::prompt::TAG);
    assert_eq!(buffer[(column("@2"), row)].fg, super::prompt::ASSIGNEE);

    // a file with neither says nothing rather than `#0 @0`
    // looked for in the list's own cell, because the row carries the file view
    // beside it and that one is full of `#tags`
    let plain = lines
        .iter()
        .find(|line| line.contains("b/TODOs.md  0/1"))
        .expect("the other file");
    let cell = plain.split('│').nth(1).unwrap_or_default();
    assert!(!cell.contains('#'), "{cell:?}");
    assert!(!cell.contains('@'), "{cell:?}");
}

#[test]
fn the_folders_are_grey_and_the_file_name_is_not() {
    // two files, so the one being measured is not the one under the band
    let (_tmp, app) = app(&[
        ("app/TODOs.md", "- [ ] z\n"),
        ("web/src/TODOs.md", "- [ ] a\n"),
    ]);
    let lines = screen(&app);
    // the list row, not the view's title, which names the same file
    let at = lines
        .iter()
        .position(|line| line.contains("web/src/TODOs.md") && line.contains("0/1"))
        .expect("the file is listed");
    let text = &lines[at];
    let column =
        |needle: &str| u16::try_from(text[..text.find(needle).unwrap()].chars().count()).unwrap();
    let buffer = terminal(&app, WIDTH, HEIGHT).backend().buffer().clone();
    let row = u16::try_from(at).unwrap();
    // the folder is dim, the name is not, and the numbers are dim again
    assert!(buffer[(column("web"), row)]
        .modifier
        .contains(Modifier::DIM));
    assert!(!buffer[(column("TODOs"), row)]
        .modifier
        .contains(Modifier::DIM));
}

#[test]
fn the_file_panel_scrolls_to_the_file_that_is_selected() {
    // one file per folder, and more of them than the sidebar has rows
    let scratch = tempfile::tempdir().unwrap();
    for number in 0..24 {
        let folder = scratch.path().join(format!("f{number:02}"));
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("TODOs.md"), "- [ ] a\n").unwrap();
    }
    let mut app = crate::state::App::open(scratch.path()).unwrap();
    app.pane = crate::state::action::Pane::Files;

    let lines = screen_at(&app, 80, 40);
    let text = lines.join("\n");
    assert!(text.contains("f00/"), "{text}");
    assert!(!text.contains("f23/"), "the list overflows: {text}");

    // `G` goes to the last file, and the list follows it
    press(&mut app, ratatui::crossterm::event::KeyCode::Char('G'));
    let lines = screen_at(&app, 80, 40);
    let text = lines.join("\n");
    let buffer = terminal(&app, 80, 40).backend().buffer().clone();
    // the list row, not the view's title: the counts are only in the list
    let row = lines
        .iter()
        .position(|line| line.contains("f23/TODOs.md  0/1"))
        .unwrap_or_else(|| panic!("the last file is on screen: {text}"));
    let at = lines[row].find("f23").unwrap();
    let column = u16::try_from(lines[row][..at].chars().count()).unwrap();
    assert_eq!(
        buffer[(column, u16::try_from(row).unwrap())].bg,
        super::consts::SELECTED,
        "the last file is on screen, under the cursor: {text}"
    );
    assert!(!text.contains("f00/"), "and the top scrolled away: {text}");
}

#[test]
fn the_selected_task_scrolls_into_view() {
    use std::fmt::Write as _;

    let mut source = String::new();
    for index in 0..60 {
        let _ = writeln!(source, "- [ ] task number {index}");
    }
    let (_tmp, mut app) = app(&[("TODOs.md", &source)]);
    app.pane = crate::state::action::Pane::Todos;
    app.task_idx = 39;
    let lines = screen_at(&app, WIDTH, HEIGHT);
    assert!(
        lines.iter().any(|line| line.contains("task number 39")),
        "the selected task is on screen"
    );
}

#[test]
fn the_outline_selection_scrolls_the_file_view() {
    use std::fmt::Write as _;

    let mut source = String::from("# top\n\n");
    for index in 0..40 {
        let _ = writeln!(source, "- [ ] task {index}");
    }
    source.push_str("\n## bottom\n\n- [ ] last\n");
    let (_tmp, mut app) = app(&[("TODOs.md", &source)]);
    app.pane = crate::state::action::Pane::Outline;
    app.outline_idx = 1;
    let lines = screen_at(&app, WIDTH, HEIGHT);
    assert!(
        lines.iter().any(|line| line.contains("## bottom")),
        "the heading the outline points at is on screen:\n{}",
        lines.join("\n")
    );
}

#[test]
fn the_file_panel_scrolls_to_the_collections_too() {
    // more files than rows, so the list has to move to show the tail of it
    let mut files = Vec::new();
    for index in 0..30 {
        files.push((
            Box::leak(format!("f{index:02}/TODOs.md").into_boxed_str()) as &'static str,
            "- [ ] a #home\n",
        ));
    }
    let (_tmp, mut app) = app(&files);
    app.pane = crate::state::action::Pane::Files;

    // the cursor walks down to the collection, which is the last row
    app.row_idx = app.rows.len() - 1;
    let lines = screen_at(&app, 80, 24);
    let text = lines.join("\n");
    assert!(
        text.contains("#home"),
        "the row the cursor is on is on screen:\n{text}"
    );
    // and the scroll followed the cursor: the rows above it were left behind
    assert!(lines.iter().any(|line| line.contains("f00/TODOs.md")), "{text}");
    assert!(!lines.iter().any(|line| line.contains("f01/TODOs.md")), "{text}");
}

#[test]
fn the_files_panel_lists_collections_after_the_files() {
    let (_tmp, app) = app(&[
        (
            "a/TODOs.md",
            "- [ ] (A) pay rent #home @sam\n- [x] ship it #work\n",
        ),
        ("b/TODOs.md", "- [ ] buy milk #home @sam\n"),
    ]);
    let rows = files_panel(&app);
    assert_eq!(
        rows,
        vec![
            "a/TODOs.md  1/2  #2  @1",
            "b/TODOs.md  0/1  #1  @1",
            // done/all, like a file: a collection is read the same way
            "#home  0/2",
            "#work  1/1",
            "@sam  0/2",
        ],
        "{rows:?}"
    );

    // the names wear the colours the todo panel gives them, and the count is dim
    let lines = screen(&app);
    let at = lines
        .iter()
        .position(|line| line.contains("#home  0/2"))
        .unwrap();
    let text = &lines[at];
    let column =
        |needle: &str| u16::try_from(text[..text.find(needle).unwrap()].chars().count()).unwrap();
    let buffer = terminal(&app, WIDTH, HEIGHT).backend().buffer().clone();
    let row = u16::try_from(at).unwrap();
    let dim = |x: u16| buffer[(x, row)].modifier.contains(Modifier::DIM);
    assert_eq!(buffer[(column("#home"), row)].fg, super::prompt::TAG);
    assert!(!dim(column("#home")));
    assert!(dim(column("0/2")), "the count is not the name");
}

#[test]
fn a_highlighted_todo_keeps_its_tags_and_people_as_names() {
    let (_tmp, app) = app(&[("TODOs.md", "- [ ] (A) pay rent #home @sam\n")]);
    let lines = screen(&app);
    let row = lines
        .iter()
        .position(|line| line.contains("pay rent #home @sam"))
        .expect("the todo is on screen");
    let text = &lines[row];
    let column =
        |needle: &str| u16::try_from(text[..text.find(needle).unwrap()].chars().count()).unwrap();
    let buffer = terminal(&app, WIDTH, HEIGHT).backend().buffer().clone();
    let row = u16::try_from(row).unwrap();

    // the words are on the light band
    assert_eq!(buffer[(column("pay"), row)].bg, super::consts::SELECTED);
    // and a tag and a person keep their own colour, in white, so they still read as
    // names instead of being flattened into the row
    for (needle, colour) in [
        ("#home", super::prompt::TAG),
        ("@sam", super::prompt::ASSIGNEE),
    ] {
        let cell = &buffer[(column(needle), row)];
        assert_eq!(cell.bg, colour, "{needle}");
        assert_eq!(cell.fg, ratatui::style::Color::White, "{needle}");
    }
}

#[test]
fn the_search_list_scrolls_to_the_hit_that_is_selected() {
    let source: String = (0..30).fold(String::new(), |mut source, index| {
        use std::fmt::Write as _;
        let _ = writeln!(source, "- [ ] milk {index}");
        source
    });
    let (_tmp, mut app) = app(&[("TODOs.md", &source)]);
    press(&mut app, ratatui::crossterm::event::KeyCode::Char('f'));
    // nothing typed: every todo matches, so the list is longer than the popover
    let first = screen_at(&app, 80, 24).join("\n");
    assert!(first.contains("milk 0"), "{first}");
    assert!(first.contains("30 matches"), "{first}");

    for _ in 0..29 {
        press_with(
            &mut app,
            ratatui::crossterm::event::KeyCode::Down,
            ratatui::crossterm::event::KeyModifiers::NONE,
        );
    }
    let lines = screen_at(&app, 80, 24);
    let text = lines.join("\n");
    // the last hit is the one selected, and it is in the window
    assert!(text.contains("milk 29"), "{text}");
    // the top of the list has scrolled away rather than the cursor
    assert!(!text.contains("milk 0 "), "{text}");
}

#[test]
fn a_collection_groups_its_todos_under_the_file_they_come_from() {
    let (_tmp, mut app) = app(&[
        (
            "a/TODOs.md",
            "# a\n\n- [ ] pay rent #home\n- [x] ship it #home\n",
        ),
        ("b/TODOs.md", "# b\n\n- [ ] buy milk #home\n"),
    ]);
    app.pane = crate::state::action::Pane::Files;
    app.row_idx = app.files.len();
    press(&mut app, ratatui::crossterm::event::KeyCode::Right);
    let lines = screen_at(&app, 100, 24);
    // the file is a heading over its todos, once — not a prefix on every one. The
    // left column lists the files too, so the count is only of the collection panel
    let top = lines.iter().find(|line| line.contains("#home todos")).unwrap();
    let right_at = top
        .chars()
        .enumerate()
        .filter(|(_, c)| *c == '╭')
        .nth(1)
        .map(|(at, _)| at)
        .expect("the second frame is the collection");
    let on_the_right = |line: &str| line.chars().skip(right_at).collect::<String>();
    let right = |needle: &str| {
        lines
            .iter()
            .position(|line| on_the_right(line).contains(needle))
            .unwrap_or_else(|| panic!("{needle} is in the collection:\n{}", lines.join("\n")))
    };
    let row_text = |at: usize| on_the_right(&lines[at]);
    assert_eq!(
        lines
            .iter()
            .filter(|line| on_the_right(line).contains("a/TODOs.md"))
            .count(),
        1,
        "{}",
        lines.join("\n")
    );
    let first = right("a/TODOs.md");
    let second = right("b/TODOs.md");
    // its todos come under it, drawn as the file has them
    assert!(row_text(first + 1).contains("3 - [ ] pay rent #home"), "{}", row_text(first + 1));
    assert!(row_text(first + 2).contains("4 - [x] ship it #home"), "{}", row_text(first + 2));
    // and a blank row separates the next file's
    let above = row_text(second - 1);
    assert_eq!(above.trim_matches(|c| c == '│' || c == ' '), "", "{above:?}");
    assert!(row_text(second + 1).contains("3 - [ ] buy milk #home"), "{}", row_text(second + 1));
}

#[test]
fn a_collection_hides_the_index_because_a_collection_has_no_headings() {
    let (_tmp, mut app) = app(&[(
        "a/TODOs.md",
        "# a\n\n## later\n\n- [ ] pay rent #home\n",
    )]);
    app.pane = crate::state::action::Pane::Files;
    // the file panel, with the file's own headings beside it
    assert!(screen(&app).iter().any(|line| line.contains("╭ [2] index")));

    // the cursor on the collection row previews it, and the index goes: the
    // headings on screen would be the open file's, not the collection's
    press(&mut app, ratatui::crossterm::event::KeyCode::Down);
    let lines = screen_at(&app, 96, 24);
    assert!(
        !lines.iter().any(|line| line.contains("index")),
        "{}",
        lines.join("\n")
    );
    // files and history are still there, and the collection takes the right side
    assert!(lines.iter().any(|line| line.contains(" files ")), "{}", lines.join("\n"));
    assert!(lines.iter().any(|line| line.contains(" history")));
    assert!(lines.iter().any(|line| line.contains("#home todos")));

    // and back on a file row the index returns, with the file's headings in it
    press(&mut app, ratatui::crossterm::event::KeyCode::Up);
    let lines = screen_at(&app, 96, 24);
    assert!(lines.iter().any(|line| line.contains("╭ [2] index")));
    assert!(lines.iter().any(|line| line.contains("## later")));
}

#[test]
fn a_collected_todo_shows_what_is_being_typed_into_it() {
    let (_tmp, mut app) = app(&[("a/TODOs.md", "- [ ] pay rent #home\n")]);
    app.pane = crate::state::action::Pane::Files;
    app.row_idx = app.files.len();
    press(&mut app, ratatui::crossterm::event::KeyCode::Right);
    press(&mut app, ratatui::crossterm::event::KeyCode::Char('e'));
    for character in " now".chars() {
        press(&mut app, ratatui::crossterm::event::KeyCode::Char(character));
    }
    let text = screen_at(&app, 96, 20).join("\n");
    assert!(text.contains("pay rent #home now"), "{text}");
}

#[test]
fn a_collection_highlights_the_todo_and_not_the_file_it_came_from() {
    let (_tmp, mut app) = app(&[("a/TODOs.md", "# a\n\n- [ ] pay rent #home\n")]);
    app.pane = crate::state::action::Pane::Files;
    app.row_idx = app.files.len();
    press(&mut app, ratatui::crossterm::event::KeyCode::Right);
    let lines = screen_at(&app, 100, 24);
    let buffer = terminal(&app, 100, 24).backend().buffer().clone();
    let row_of = |needle: &str| {
        u16::try_from(
            lines
                .iter()
                .position(|line| line.contains(needle))
                .expect("on screen"),
        )
        .unwrap()
    };
    let column = |row: u16, needle: &str| {
        let line = &lines[usize::from(row)];
        u16::try_from(line[..line.find(needle).unwrap()].chars().count()).unwrap()
    };

    // where it lives is a heading of its own: no band on it
    let path_row = row_of("a/TODOs.md");
    assert_ne!(
        buffer[(column(path_row, "a/TODOs.md"), path_row)].bg,
        super::consts::SELECTED,
        "the path is not the todo"
    );
    // the todo is banded, from the box the file has it in — the number gutter and
    // the path above stay plain
    let todo_row = row_of("pay rent #home");
    assert_eq!(
        buffer[(column(todo_row, "- ["), todo_row)].bg,
        super::consts::SELECTED
    );
    // and its tag is a name on its own cyan
    assert_eq!(
        buffer[(column(todo_row, "#home"), todo_row)].bg,
        super::prompt::TAG
    );
}

#[test]
fn a_collection_shows_its_heading_its_files_and_its_own_tally() {
    let (_tmp, mut app) = app(&[
        ("a/TODOs.md", "# a\n\n- [x] pay rent #home\n"),
        ("b/TODOs.md", "# b\n\n- [ ] buy milk #home\n"),
    ]);
    app.pane = crate::state::action::Pane::Files;
    // the collection is drawn as soon as its row has the cursor, before `→`
    app.row_idx = app.files.len() - 1;
    press(&mut app, ratatui::crossterm::event::KeyCode::Down);
    let lines = screen_at(&app, 100, 24);
    let text = lines.join("\n");
    assert!(
        text.contains("# tag home todo collection"),
        "a preview: {text}"
    );
    let text = lines.join("\n");
    // the frame says what is open, and the first row says what it is
    assert!(text.contains("#home todos"), "{text}");
    assert!(text.contains("# tag home todo collection"), "{text}");
    // each file is a heading over its own todos, because an edit there writes there
    assert!(text.contains("a/TODOs.md"), "{text}");
    assert!(text.contains("b/TODOs.md"), "{text}");
    assert!(text.contains("3 - [x] pay rent #home"), "{text}");
    assert!(text.contains("3 - [ ] buy milk #home"), "{text}");
    // and the tally is the collection's own: no tags, no people
    let last = lines.iter().rev().find(|line| line.contains('╯')).unwrap();
    assert!(last.trim_end().ends_with("1/2 ╯"), "{last:?}");
}

#[test]
fn a_file_with_no_tasks_renders() {
    let (_tmp, app) = single("# nothing here\n");
    let lines = screen(&app);
    assert!(
        lines.iter().any(|line| line.contains("0/0")),
        "{}",
        lines.join("\n")
    );
}

#[test]
fn a_tag_is_coloured_before_the_word_ends() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
    app.pane = crate::state::action::Pane::View;
    press(&mut app, ratatui::crossterm::event::KeyCode::Char('n'));
    // typed character by character, and still in the middle of the tag
    type_text(&mut app, "ship #inf");
    let lines = screen(&app);
    let prompt_row = lines
        .iter()
        .position(|line| line.contains("ship #inf"))
        .expect("the field is on screen");
    let text = &lines[prompt_row];
    let tag = u16::try_from(text[..text.find("#inf").unwrap()].chars().count()).unwrap();
    let buffer = terminal(&app, WIDTH, HEIGHT).backend().buffer().clone();
    assert_eq!(
        buffer[(tag, u16::try_from(prompt_row).unwrap())].fg,
        super::prompt::TAG
    );
}

#[test]
fn the_prompt_colours_what_the_parser_sees() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
    app.pane = crate::state::action::Pane::Todos;
    app.mode = Mode::Adding(Prompt::new("ship #x @y"));
    let terminal = terminal(&app, WIDTH, HEIGHT);
    let buffer = terminal.backend().buffer();
    let lines = screen(&app);
    let prompt_row = lines
        .iter()
        .position(|line| line.contains("ship #x @y"))
        .expect("the prompt is on screen");
    let row = u16::try_from(prompt_row).unwrap();
    let text = &lines[prompt_row];
    // a column, not a byte: the frame's `│` is three bytes wide
    let column =
        |needle: &str| u16::try_from(text[..text.find(needle).unwrap()].chars().count()).unwrap();
    let (tag, who) = (column("#x"), column("@y"));
    assert_eq!(buffer[(tag, row)].fg, super::prompt::TAG);
    assert_eq!(buffer[(who, row)].fg, super::prompt::ASSIGNEE);
}

#[test]
fn history_rows_are_dots_that_hollow_when_reverted() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
    app.pane = crate::state::action::Pane::Todos;
    app.press(ratatui::crossterm::event::KeyCode::Char('x'));
    assert!(
        texts(&app).contains("● check todo · a"),
        "{}",
        texts(&app)
    );

    app.pane = crate::state::action::Pane::History;
    app.press(ratatui::crossterm::event::KeyCode::Char('x'));
    assert!(
        texts(&app).contains("○ check todo · a"),
        "{}",
        texts(&app)
    );
}

#[test]
fn a_dependency_line_runs_past_the_writes_between_it() {
    let (_tmp, mut app) = app(&[("TODOs.md", "# h\n\n- [ ] a\n- [ ] b\n")]);
    app.pane = crate::state::action::Pane::View;
    app.press(ratatui::crossterm::event::KeyCode::Char('x'));
    app.press(ratatui::crossterm::event::KeyCode::Down);
    app.press(ratatui::crossterm::event::KeyCode::Char('x'));
    app.press(ratatui::crossterm::event::KeyCode::Up);
    app.press(ratatui::crossterm::event::KeyCode::Char('x'));

    let text = screen(&app).join("\n");
    // newest first: uncheck a, check b, check a. The line from the uncheck
    // runs down past an unrelated write to the check it undoes.
    assert!(text.contains("╭─● uncheck todo · a"), "{text}");
    assert!(text.contains("│ ● check todo · b"), "{text}");
    assert!(text.contains("╰─● check todo · a"), "{text}");
}

#[test]
fn the_graph_runs_a_line_from_each_write_down_to_the_one_it_undoes() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] (A) write the README\n")]);
    app.pane = crate::state::action::Pane::View;
    for key in ['x', 'B', 'x', 'B'] {
        app.press(ratatui::crossterm::event::KeyCode::Char(key));
    }
    let lines = screen_at(&app, 160, 40);
    // newest first, two chains crossing: the priority pair in lane 0, the
    // checkbox pair in lane 1. `╭` leaves a row downwards, `╰` arrives and runs
    // into the one it lands on, and `┼` is a lane running through a run.
    let expected = [
        "│ ╭──● clear prio · write the README",
        "│ │╭─● uncheck todo · write the README",
        "│ ╰┼─● set prio B · write the README",
        "│  ╰─● check todo · write the README",
    ];
    for want in expected {
        assert!(
            lines.iter().any(|line| line.contains(want)),
            "{want}\n{}",
            lines.join("\n")
        );
    }

    // the gutter is one width for every row, so no label is pushed sideways
    let columns: Vec<usize> = expected
        .iter()
        .map(|want| {
            let row = lines.iter().find(|line| line.contains(want)).unwrap();
            let at = row.find('●').unwrap();
            unicode_width::UnicodeWidthStr::width(&row[..at])
        })
        .collect();
    assert!(
        columns.windows(2).all(|pair| pair[0] == pair[1]),
        "labels line up: {columns:?}"
    );
}

#[test]
fn a_row_that_cannot_be_toggled_says_why_under_itself() {
    let (_tmp, mut app) = app(&[("TODOs.md", "# h\n\n- [ ] a\n")]);
    app.pane = crate::state::action::Pane::View;
    for _ in 0..2 {
        app.press(ratatui::crossterm::event::KeyCode::Char('x'));
    }
    // undo the check: the uncheck hangs on it, so it cannot be redone yet
    app.pane = crate::state::action::Pane::History;
    app.history_idx = 1;
    app.press(ratatui::crossterm::event::KeyCode::Char('x'));
    app.press(ratatui::crossterm::event::KeyCode::Up);

    let lines = screen_at(&app, 120, 40);
    let text = lines.join("\n");
    let row = lines
        .iter()
        .position(|line| line.contains("! enable the write below"))
        .unwrap_or_else(|| panic!("{text}"));
    // it sits directly under the selected row, and the keys row is untouched
    assert!(lines[row - 1].contains("○ uncheck todo · a"), "{text}");
    let status = lines[lines.len() - 2..].join("\n");
    assert!(!status.contains("enable the write"), "{status}");
    assert!(
        status.contains("↑/↓ nav"),
        "the keys are still there: {status}"
    );
}

#[test]
fn a_write_and_the_undo_of_it_are_drawn_as_a_pair() {
    let (_tmp, mut app) = app(&[("TODOs.md", "# h\n\n- [ ] a\n")]);
    app.pane = crate::state::action::Pane::View;
    app.press(ratatui::crossterm::event::KeyCode::Char('x'));
    app.press(ratatui::crossterm::event::KeyCode::Char('x'));

    let text = screen(&app).join("\n");
    assert!(text.contains("╭─● uncheck todo · a"), "{text}");
    assert!(
        text.contains("╰─● check todo · a"),
        "the line ends at the write it undoes: {text}"
    );

    // the arrow takes a column, so no label is pushed sideways by it. Columns
    // are cells, not bytes: the box and arrow glyphs are three bytes each.
    let column = |needle: &str| {
        screen(&app).iter().find_map(|line| {
            line.find(needle)
                .map(|index| unicode_width::UnicodeWidthStr::width(&line[..index]))
        })
    };
    assert_eq!(column("[x] uncheck"), column("[x] check"), "labels line up");
}

#[test]
fn only_a_file_with_history_shows_an_h() {
    let (_tmp, mut app) = app(&[
        ("a/TODOs.md", "- [ ] a\n"),
        ("b/TODOs.md", "- [ ] b\n"),
        ("c/TODOs.md", "- [ ] c\n"),
    ]);
    app.pane = crate::state::action::Pane::Files;
    // nothing has been written anywhere yet, so no file says it has
    for row in files_panel(&app) {
        assert!(!row.contains("  h"), "no h without history: {row:?}");
    }

    // one check in `a`, and only that file says so
    app.file_idx = 0;
    app.pane = crate::state::action::Pane::View;
    press(&mut app, ratatui::crossterm::event::KeyCode::Char('x'));
    app.pane = crate::state::action::Pane::Files;
    let rows = files_panel(&app);
    assert!(rows[0].contains("  h1"), "{rows:?}");
    for row in &rows[1..] {
        assert!(!row.contains("  h"), "still nothing here: {row:?}");
    }

    // and it is orange, like a warning: the one thing on the row worth a
    // second look — seen on a row that is not under the band, which flattens it
    app.row_idx = 2;
    app.file_idx = 2;
    let lines = screen(&app);
    let at = lines
        .iter()
        .position(|line| line.contains("h1"))
        .expect("the indicator is on screen");
    let column = u16::try_from(lines[at][..lines[at].find("h1").unwrap()].chars().count()).unwrap();
    let buffer = terminal(&app, WIDTH, HEIGHT).backend().buffer().clone();
    assert_eq!(
        buffer[(column, u16::try_from(at).unwrap())].fg,
        super::consts::WARNING
    );

    // a second write in the same file counts up
    app.file_idx = 1;
    app.pane = crate::state::action::Pane::View;
    press(&mut app, ratatui::crossterm::event::KeyCode::Char('x'));
    app.pane = crate::state::action::Pane::Files;
    let rows = files_panel(&app);
    assert!(rows[0].contains("  h1"), "{rows:?}");
    assert!(rows[1].contains("  h1"), "{rows:?}");
    assert!(!rows[2].contains("  h"), "{rows:?}");
}

#[test]
fn the_status_bar_is_general_keys_then_the_keys_of_whoever_has_the_keyboard() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
    app.pane = crate::state::action::Pane::Files;
    let lines = screen(&app);
    let rows = &lines[lines.len() - 2..];
    assert!(
        rows[0].contains("f search") && rows[0].contains("q quit"),
        "{rows:?}"
    );
    assert!(
        !rows[0].contains('▸'),
        "where you are is the panel's dot, not a word here: {rows:?}"
    );
    assert!(rows[1].contains("↑/↓ nav"), "{rows:?}");

    // a modal takes the second row over; the shared keys stay where they are
    press(&mut app, ratatui::crossterm::event::KeyCode::Char('n'));
    let lines = screen(&app);
    let rows = &lines[lines.len() - 2..];
    assert!(rows[0].contains("f search"), "{rows:?}");
    assert!(rows[1].contains("esc cancel · enter create"), "{rows:?}");
}

#[test]
fn the_new_file_field_shows_the_whole_path_and_only_the_middle_is_typed() {
    let (_tmp, mut app) = app(&[("web/TODOs.md", "- [ ] a\n")]);
    app.pane = crate::state::action::Pane::Files;
    press(&mut app, ratatui::crossterm::event::KeyCode::Char('n'));
    for character in "src".chars() {
        press(
            &mut app,
            ratatui::crossterm::event::KeyCode::Char(character),
        );
    }
    let text = texts(&app);
    assert!(text.contains("./web/src/TODOs.md"), "{text}");

    // the frame is grey and the middle is not: `./` and `/TODOs.md` cannot be
    // typed, so they do not look like something that can be
    let lines = screen(&app);
    let at = lines
        .iter()
        .position(|line| line.contains("./web/src/TODOs.md"))
        .unwrap();
    let column = |needle: &str| {
        u16::try_from(lines[at][..lines[at].find(needle).unwrap()].chars().count()).unwrap()
    };
    let buffer = terminal(&app, WIDTH, HEIGHT).backend().buffer().clone();
    let row = u16::try_from(at).unwrap();
    assert!(buffer[(column("./"), row)].modifier.contains(Modifier::DIM));
    assert!(!buffer[(column("web"), row)]
        .modifier
        .contains(Modifier::DIM));
    assert!(buffer[(column("TODOs.md"), row)]
        .modifier
        .contains(Modifier::DIM));

    // home stops at the start of the middle, not inside the frame
    press(&mut app, ratatui::crossterm::event::KeyCode::Home);
    press(&mut app, ratatui::crossterm::event::KeyCode::Char('x'));
    let text = texts(&app);
    assert!(text.contains("./xweb/src/TODOs.md"), "{text}");

    // and with nothing typed the path is the root's own file
    press(&mut app, ratatui::crossterm::event::KeyCode::End);
    for _ in 0..8 {
        press(&mut app, ratatui::crossterm::event::KeyCode::Backspace);
    }
    let text = texts(&app);
    assert!(text.contains("./TODOs.md"), "{text}");
}

#[test]
fn the_conflict_question_is_a_modal_with_two_ways_out() {
    let (_tmp, mut app) = app(&[("web/TODOs.md", "- [ ] a\n")]);
    app.pane = crate::state::action::Pane::View;
    press(&mut app, ratatui::crossterm::event::KeyCode::Char('x'));
    app.conflict();
    let lines = screen(&app);
    let text = lines.join("\n");
    assert!(text.contains("╭ changed on disk"), "{text}");
    assert!(text.contains("web/TODOs.md"), "it names the file: {text}");
    assert!(text.contains("Y keep mine"), "{text}");
    assert!(text.contains("t take theirs"), "{text}");
    // the bar says the same two keys, and the panel keys are gone
    let bar = &lines[lines.len() - 2..];
    assert!(bar[1].contains("Y keep mine"), "{bar:?}");
    assert!(
        !bar[1].contains("x/space"),
        "the panel is not in charge: {bar:?}"
    );
}

#[test]
fn the_delete_question_is_a_modal_and_the_bar_keeps_its_two_rows() {
    let (_tmp, mut app) = app(&[("TODOs.md", "# inbox\n\n- [ ] pay rent #home\n")]);
    app.pane = crate::state::action::Pane::View;
    press(&mut app, ratatui::crossterm::event::KeyCode::Char('d'));
    let lines = screen(&app);
    let text = lines.join("\n");
    assert!(text.contains("╭ delete todo"), "a framed question: {text}");
    assert!(text.contains("pay rent"), "{text}");
    assert!(
        !lines[lines.len() - 2].contains("delete"),
        "not a line in the bar: {text}"
    );
    let status = &lines[lines.len() - 2..];
    assert!(status[0].contains("f search"), "{status:?}");
    // one rule for the question: any key cancels, only the capital deletes
    assert!(status[1].contains("esc cancel · Y delete"), "{status:?}");
}

#[test]
fn the_scan_warnings_live_in_the_files_panel_not_the_status_bar() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("TODOs.md"), "- [ ] a\n").unwrap();
    // one level past the limit, whatever the limit is
    let too_deep = "d/".repeat(aricma_todo_lib::limits::MAX_DEPTH + 1);
    let deep = tmp.path().join(too_deep.trim_end_matches('/'));
    std::fs::create_dir_all(&deep).unwrap();
    std::fs::write(deep.join("TODOs.md"), "- [ ] too deep\n").unwrap();
    let app = crate::state::App::open(tmp.path()).unwrap();
    assert_eq!(app.warnings.len(), 1, "{:?}", app.warnings);

    let lines = screen(&app);
    let text = lines.join("\n");
    // the line wraps in a sidebar, so the check is a phrase that stays whole
    assert!(
        text.contains("10 levels"),
        "the warning is on screen: {text}"
    );

    // the status bar keeps its hints: the two rows above the bottom
    let status = lines[lines.len() - 2..].join("\n");
    assert!(!status.contains("10 levels"), "{status}");
    assert!(
        status.contains("↑/↓ nav"),
        "the panel's keys are back: {status}"
    );

    // and the warning is inside the files panel, which starts at the top left
    let first = lines
        .iter()
        .position(|line| line.contains("[warn]"))
        .unwrap();
    assert!(
        lines[first].starts_with('│'),
        "inside a panel: {:?}",
        lines[first]
    );
    // a log line, and in the warning colour: orange, not red and not a whisper
    assert!(lines[first].contains("[warn]"), "{:?}", lines[first]);
    let column = lines[first][..lines[first].find("[warn]").unwrap()]
        .chars()
        .count();
    let buffer = terminal(&app, WIDTH, HEIGHT).backend().buffer().clone();
    assert_eq!(
        buffer[(
            u16::try_from(column).unwrap(),
            u16::try_from(first).unwrap()
        )]
            .fg,
        super::consts::WARNING
    );
}

#[test]
fn the_search_popover_lists_the_matching_todos() {
    let (_tmp, mut app) = app(&[(
        "TODOs.md",
        "# milk\n\n- [ ] email the landlord\n- [ ] buy milk\n",
    )]);
    app.open_search();
    for character in "milk".chars() {
        app.press(ratatui::crossterm::event::KeyCode::Char(character));
    }
    let lines = screen(&app);
    let text = lines.join("\n");
    assert!(text.contains("search"), "{text}");
    assert!(text.contains("buy milk"), "{text}");
    assert!(
        text.contains("TODOs.md:4"),
        "the result names its line: {text}"
    );
    // the popover is only as tall as its hits, so the file behind it is on screen:
    // "only matches" is a claim about the box, not about the screen
    let top = lines
        .iter()
        .position(|line| line.contains("search · 1 match"))
        .expect("the popover");
    // the box's own bottom: the same column as its top-left corner, so a panel's
    // border underneath it is not mistaken for it
    let column = lines[top].chars().position(|c| c == '╭').expect("a corner");
    let bottom = top
        + lines[top..]
            .iter()
            .position(|line| line.chars().nth(column) == Some('╰'))
            .expect("its bottom border");
    let inside = lines[top..=bottom].join("\n");
    assert!(
        !inside.contains("email the landlord"),
        "only matches are listed: {inside}"
    );
    assert!(inside.contains("buy milk"), "{inside}");
    assert!(bottom - top < 8, "and the box is short: {inside}");
    assert!(
        lines.iter().any(|line| line.contains("files")),
        "the screen is still drawn under it"
    );
    // the bar's second row belongs to whoever has the keyboard, and that is the
    // popover now: its own keys, not the panel's that are still showing behind it
    let bar = lines.last().expect("a status row").clone();
    assert!(bar.contains("esc cancel"), "{bar:?}");
    assert!(bar.contains("ctrl+u clear"), "{bar:?}");
}

#[test]
fn a_search_offers_enter_only_while_it_has_something_to_open() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] buy milk\n")]);
    press(&mut app, ratatui::crossterm::event::KeyCode::Char('f'));
    // every todo matches an empty query, so there is a hit to go to
    let text = screen(&app).join("\n");
    assert!(text.contains("esc cancel · enter jump"), "{text}");

    for character in "zzz".chars() {
        press(&mut app, ratatui::crossterm::event::KeyCode::Char(character));
    }
    let text = screen(&app).join("\n");
    assert!(text.contains(crate::text::NO_MATCH), "{text}");
    assert!(text.contains(" esc cancel "), "{text}");
    assert!(
        !text.contains("enter jump"),
        "nothing to open, so the key is not offered: {text}"
    );
}

#[test]
fn the_keys_page_breathes() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
    press(&mut app, ratatui::crossterm::event::KeyCode::Char('?'));
    let lines = screen_at(&app, 80, 24);

    // two columns of air before the first one: the frame's own padding, and one more
    let general = lines
        .iter()
        .find(|line| line.contains("general"))
        .expect("a column");
    assert!(general.contains("│  general"), "{general:?}");

    // and no row wraps: a wrapped line puts the words on the line under its keys,
    // which is what three narrow columns used to do
    for (keys, does) in [
        ("tab", "next section"),
        ("esc", "dismiss it"),
        ("ctrl+u", "empty the search"),
        ("↑/↓", "nav"),
        ("enter", "open file"),
    ] {
        let line = lines
            .iter()
            .find(|line| line.contains(does))
            .unwrap_or_else(|| panic!("{does} is not on the page"));
        assert!(line.contains(keys), "{keys} and {does} on one line: {line:?}");
    }

    // the widest row stops short of the border rather than under it
    let widest = lines
        .iter()
        .find(|line| line.contains("clear prio"))
        .expect("the widest row");
    assert_eq!(widest.chars().rev().nth(1), Some(' '), "{widest:?}");
}

#[test]
fn a_search_result_jumps_to_its_todo_and_its_heading() {
    let (_tmp, mut app) = app(&[(
        "TODOs.md",
        "# first\n\n- [ ] a\n\n## second\n\n- [ ] buy milk\n",
    )]);
    app.pane = crate::state::action::Pane::Files;
    app.open_search();
    for character in "milk".chars() {
        app.press(ratatui::crossterm::event::KeyCode::Char(character));
    }
    app.press(ratatui::crossterm::event::KeyCode::Enter);

    assert_eq!(app.current_task().unwrap().title, "buy milk");
    assert_eq!(app.outline_idx, 1, "the outline lands on its heading");
    assert!(!app.modal(), "and the popover closes");
    assert_eq!(
        app.pane,
        crate::state::action::Pane::View,
        "with the todo in front"
    );
}

#[test]
fn a_too_long_file_is_marked_and_not_opened() {
    let tmp = tempfile::tempdir().unwrap();
    let long: String =
        std::iter::repeat_n("- [ ] a line\n", aricma_todo_lib::limits::MAX_LINES + 1).collect();
    std::fs::write(tmp.path().join("TODOs.md"), &long).unwrap();
    std::fs::write(tmp.path().join("TASKS.md"), "- [ ] fine\n").unwrap();
    let mut app = crate::state::App::open(tmp.path()).unwrap();

    let text = screen(&app).join("\n");
    assert!(text.contains("too long"), "{text}");

    app.press(ratatui::crossterm::event::KeyCode::Down);
    assert!(
        app.message_text().contains("not opened"),
        "{:?}",
        app.message
    );
}

#[test]
fn a_tiny_terminal_renders_without_panicking() {
    let (_tmp, app) = app(&[
        ("TODOs.md", "- [ ] (A) pay rent #home @sam\n"),
        ("b/TASKS.md", "- [ ] x\n"),
    ]);
    assert_eq!(screen_at(&app, 40, 12).len(), 12);
    let (_tmp, app) = single("- [ ] (A) pay rent #home @sam\n");
    assert_eq!(screen_at(&app, 20, 6).len(), 6);
}

/// A whole screen, committed as a snapshot: layout regressions that leave
/// the text intact still fail here.
#[test]
fn the_whole_screen_at_80x24() {
    let (_tmp, mut app) = app(&[
        (
            "a/TODOs.md",
            "# inbox\n\n- [ ] (A) pay rent #home @sam\n- [x] ship it\n\n## later\n",
        ),
        ("b/TASKS.md", "- [ ] (B) another #one\n"),
    ]);
    app.pane = crate::state::action::Pane::Todos;
    app.press(ratatui::crossterm::event::KeyCode::Char('x'));
    app.pane = crate::state::action::Pane::History;
    app.press(ratatui::crossterm::event::KeyCode::Char('x'));
    insta::assert_snapshot!(masked(&app, &screen(&app).join("\n")));
}

#[test]
fn the_whole_screen_at_120x40() {
    let (_tmp, app) = app(&[(
        "TODOs.md",
        "# inbox\n\n- [ ] (B) @dave fix you #worst-bug-of-the-century\n- [ ] plain task\n- [x] done task\n\n## later\n\n- [ ] last\n",
    )]);
    insta::assert_snapshot!(masked(&app, &screen_at(&app, 120, 40).join("\n")));
}

#[test]
fn a_view_focused_screen_at_80x24() {
    let (_tmp, mut app) = app(&[(
        "TODOs.md",
        "# inbox\n\n- [ ] (A) pay rent #home @sam\n- [x] ship it\n\n## later\n",
    )]);
    app.press(ratatui::crossterm::event::KeyCode::Right);
    insta::assert_snapshot!(masked(&app, &screen(&app).join("\n")));
}

#[test]
fn a_wrapped_todo_at_80x24() {
    let (_tmp, app) = single(
        "- [ ] (A) @dave fix the worst bug of the century before the release #bug #infra\n\
         - [x] short one\n",
    );
    insta::assert_snapshot!(masked(&app, &screen(&app).join("\n")));
}

#[test]
fn a_single_file_at_80x24() {
    let (_tmp, app) =
        single("# inbox\n\n- [ ] (A) pay rent #home @sam\n- [x] ship it\n\n## later\n");
    insta::assert_snapshot!(masked(&app, &screen(&app).join("\n")));
}

#[test]
fn the_cursor_follows_the_text_into_the_next_row_of_the_field() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
    app.pane = crate::state::action::Pane::View;
    press(&mut app, ratatui::crossterm::event::KeyCode::Char('n'));
    let cursor_of = |app: &crate::state::App| {
        let mut terminal = terminal(app, WIDTH, HEIGHT);
        terminal.get_cursor_position().expect("a cursor").y
    };
    type_text(&mut app, "short");
    let first = cursor_of(&app);

    // enough text to wrap onto a second row: the cursor goes with it
    type_text(
        &mut app,
        " and then a great deal more of it, enough to fill another line",
    );
    let later = cursor_of(&app);
    assert!(
        later > first,
        "the cursor moved down with the text: {first} -> {later}"
    );

    // and the field grew a row for the rule under the text, so the cursor is
    // still inside it
    let lines = screen(&app);
    let bottom = lines
        .iter()
        .position(|line| line.contains("esc cancel · enter add"))
        .expect("the field is on screen");
    assert!(usize::from(later) < bottom, "the cursor stays in the field");
}













#[test]
fn a_panel_shows_a_thumb_only_when_it_has_more_rows_than_room() {
    let names: Vec<String> = (1..=30).map(|n| format!("d{n}/TODOs.md")).collect();
    let files: Vec<(&str, &str)> = names.iter().map(|n| (n.as_str(), "- [ ] a\n")).collect();
    let (_tmp, many) = app(&files);
    let text = screen(&many).join("\n");
    // a thumb on the files panel's inner right edge, and a track that is not drawn
    assert!(text.contains('▐'), "no thumb in:\n{text}");

    let (_tmp, small) = app(&[("TODOs.md", "- [ ] a\n")]);
    let text = screen(&small).join("\n");
    assert!(!text.contains('▐'), "a thumb with nothing to scroll:\n{text}");
}

#[test]
fn a_thumb_starts_at_the_top_and_stops_at_the_bottom() {
    let names: Vec<String> = (1..=30).map(|n| format!("d{n}/TODOs.md")).collect();
    let files: Vec<(&str, &str)> = names.iter().map(|n| (n.as_str(), "- [ ] a\n")).collect();
    let (_tmp, mut app) = app(&files);
    let lines = screen(&app);
    let first = lines
        .iter()
        .position(|line| line.contains("[1] files"))
        .expect("the files panel") + 1;
    assert!(lines[first].contains('▐'), "the top row:\n{}", lines.join("\n"));

    // the last row inside the panel, just above its bottom border
    press(&mut app, ratatui::crossterm::event::KeyCode::Char('G'));
    let lines = screen(&app);
    let bottom = lines
        .iter()
        .position(|line| line.contains('╰'))
        .expect("the panel's bottom border");
    assert!(
        lines[bottom - 1].contains('▐'),
        "the thumb ends at the bottom, not before it:\n{}",
        lines.join("\n")
    );
}

#[test]
fn no_panel_lets_its_band_paint_over_the_thumb() {
    // two panels that overflow at once, each with a banded row crossing its bar: the
    // files list, and the file view of the code file the cursor opened
    let mut names: Vec<String> = (1..=29).map(|n| format!("d{n}/TODOs.md")).collect();
    names.push("retry.rs".to_owned());
    let mut code = String::new();
    for n in 1..=20 {
        for line in 1..=3 {
            let _ = writeln!(code, "fn f{n}_{line}() {{}}");
        }
        code.push_str("// TODO: something to do\n");
    }
    let files: Vec<(&str, &str)> = names
        .iter()
        .map(|name| {
            let body = if name == "retry.rs" {
                code.as_str()
            } else {
                "- [ ] a\n"
            };
            (name.as_str(), body)
        })
        .collect();
    let (_tmp, mut app) = app(&files);
    // onto the code file, which puts its first todo under the band in the view
    for _ in 0..29 {
        press(&mut app, ratatui::crossterm::event::KeyCode::Down);
    }
    press(&mut app, ratatui::crossterm::event::KeyCode::Right);
    assert_eq!(app.pane, crate::state::action::Pane::View);

    let terminal = terminal(&app, WIDTH, HEIGHT);
    let buffer = terminal.backend().buffer();
    let text = screen(&app).join("\n");
    // every cell of a thumb, by column, so a thumb with a hole in it is visible
    let mut columns: std::collections::BTreeMap<u16, Vec<u16>> = std::collections::BTreeMap::new();
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            if buffer[(x, y)].symbol() != "▐" {
                continue;
            }
            assert_eq!(
                buffer[(x, y)].bg,
                ratatui::style::Color::Reset,
                "a band took the colour of the thumb at ({x}, {y}):\n{text}"
            );
            columns.entry(x).or_default().push(y);
        }
    }
    // two panels, one bar each
    assert_eq!(columns.len(), 2, "expected a bar in both panels:\n{text}");
    for (x, rows) in &columns {
        let (first, last) = (rows[0], rows[rows.len() - 1]);
        assert_eq!(
            rows.len(),
            usize::from(last - first) + 1,
            "the band cut a hole in the thumb in column {x}: {rows:?}\n{text}"
        );
    }
}

#[test]
fn the_history_band_is_on_the_label_and_not_on_the_dot() {
    let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
    app.pane = crate::state::action::Pane::View;
    press(&mut app, ratatui::crossterm::event::KeyCode::Char('x'));
    app.pane = crate::state::action::Pane::History;

    let terminal = terminal(&app, WIDTH, HEIGHT);
    let buffer = terminal.backend().buffer();
    let text = screen(&app).join("\n");
    // the write's own row: the one whose label is the entry
    let y = (0..HEIGHT)
        .find(|y| {
            (0..WIDTH)
                .map(|x| buffer[(x, *y)].symbol())
                .collect::<String>()
                .contains("check todo")
        })
        .unwrap_or_else(|| panic!("{text}"));
    let dot = (0..WIDTH)
        .find(|x| buffer[(*x, y)].symbol() == "●")
        .unwrap_or_else(|| panic!("{text}"));

    // the dot is the gutter's, so it keeps the panel's ground; the label is banded
    assert_ne!(
        buffer[(dot, y)].bg,
        crate::ui::consts::selection().bg.unwrap_or_default(),
        "the band runs over the dot:\n{text}"
    );
    assert_eq!(
        buffer[(dot + 2, y)].bg,
        crate::ui::consts::selection().bg.unwrap_or_default(),
        "the label is not banded:\n{text}"
    );
    // and the band still runs to the end of the text area, not just under the words.
    // Two cells in from the border: the frame's own padding is not the paragraph's
    let border = (dot..WIDTH)
        .find(|x| buffer[(*x, y)].symbol() == "│")
        .unwrap_or_else(|| panic!("{text}"));
    assert_eq!(
        buffer[(border - 2, y)].bg,
        crate::ui::consts::selection().bg.unwrap_or_default(),
        "the band stops short at {border}, dot {dot}\n{text}"
    );
}

#[test]
fn the_thumb_is_not_banded() {
    // a full panel whose cursor is on a row the bar runs over: the band stops before
    // the thumb, because a thumb on the selection's green is a highlight of a highlight
    let names: Vec<String> = (1..=30).map(|n| format!("d{n}/TODOs.md")).collect();
    let files: Vec<(&str, &str)> = names.iter().map(|n| (n.as_str(), "- [ ] a\n")).collect();
    let (_tmp, app) = app(&files);
    let terminal = terminal(&app, WIDTH, HEIGHT);
    let buffer = terminal.backend().buffer();
    let row = 2; // the first row inside the files panel, which is banded
    let width = usize::from(WIDTH);
    let at = |x: usize| u16::try_from(x).unwrap_or(u16::MAX);
    let thumb = (0..width)
        .find(|x| buffer[(at(*x), row)].symbol() == "▐")
        .expect("a thumb on that row");
    let band = buffer[(at(thumb - 1), row)].bg;
    assert_ne!(
        buffer[(at(thumb), row)].bg,
        band,
        "the thumb takes the band's background"
    );
    // the row really is banded, or this test proves nothing
    assert_ne!(band, ratatui::style::Color::Reset);
}

#[test]
fn a_code_file_shows_a_line_above_a_todo_and_three_below() {
    // ten lines, the todo, ten more: the fold is what keeps the todo on screen with
    // the code it is about and not the twenty lines it is not. One line above it —
    // the note is written above the change, so the change is what is below
    let mut source = String::new();
    source.push_str("fn above1() {}\n");
    source.push_str("fn above2() {}\n");
    source.push_str("fn above3() {}\n");
    source.push_str("fn above4() {}\n");
    source.push_str("fn above5() {}\n");
    source.push_str("fn above6() {}\n");
    source.push_str("fn above7() {}\n");
    source.push_str("fn above8() {}\n");
    source.push_str("fn above9() {}\n");
    source.push_str("fn above10() {}\n");
    source.push_str("// TODO: wire it up\n");
    source.push_str("fn below1() {}\n");
    source.push_str("fn below2() {}\n");
    source.push_str("fn below3() {}\n");
    source.push_str("fn below4() {}\n");
    source.push_str("fn below5() {}\n");
    source.push_str("fn below6() {}\n");
    source.push_str("fn below7() {}\n");
    source.push_str("fn below8() {}\n");
    source.push_str("fn below9() {}\n");
    source.push_str("fn below10() {}\n");
    let (_tmp, mut app) = app(&[("src/a.rs", &source)]);
    app.pane = crate::state::action::Pane::View;
    let text = screen(&app).join("\n");
    // the todo, the line it was written after, and the three it asks for
    assert!(text.contains("// TODO: wire it up"), "{text}");
    for near in ["fn above10", "fn below1", "fn below2", "fn below3"] {
        assert!(text.contains(near), "{near} is not in:\n{text}");
    }
    // the code that is not around a todo is one row, not the rest of the file
    // one `…` for the folded code, and one in the shortened path in the header
    assert!(text.matches('…').count() >= 2, "no fold in:\n{text}");
    assert!(!text.contains("fn above9("), "{text}");
    assert!(!text.contains("fn below4("), "{text}");
    assert!(!text.contains("fn below10("), "{text}");
}

#[test]
fn a_single_line_outside_a_window_is_shown_and_two_are_folded() {
    // a `…` replaces one line with one row: it hides a line and saves nothing, so the
    // line stays. Two lines is a row saved, so they go behind the fold.
    let source = "// TODO: first\nfn a() {}\nfn b() {}\nfn c() {}\nfn d() {}\n";
    let (_tmp, mut one) = app(&[("src/a.rs", source)]);
    one.pane = crate::state::action::Pane::View;
    let text = screen(&one).join("\n");
    // the window is the todo and the three lines under it; `d` is the one left over
    assert!(text.contains("fn d"), "{text}");
    // the only ellipsis on screen is the shortened path in the header
    assert_eq!(text.matches('…').count(), 1, "a fold that saves nothing:\n{text}");

    let longer = "// TODO: first\nfn a() {}\nfn b() {}\nfn c() {}\nfn d() {}\nfn e() {}\n";
    let (_tmp, mut two) = app(&[("src/b.rs", longer)]);
    two.pane = crate::state::action::Pane::View;
    let text = screen(&two).join("\n");
    assert!(!text.contains("fn e"), "two lines are a row saved:\n{text}");
    assert!(text.matches('…').count() >= 2, "no fold in:\n{text}");
}

#[test]
fn a_file_does_not_open_on_the_code_before_its_first_todo() {
    // the window is a promise: one line above the first note, however far down it is
    let mut source = String::new();
    for n in 1..=10 {
        let _ = writeln!(source, "fn above{n}() {{}}");
    }
    source.push_str("// TODO: wire it up\nfn below() {}\n");
    let (_tmp, mut app) = app(&[("src/a.rs", &source)]);
    app.pane = crate::state::action::Pane::View;
    let text = screen(&app).join("\n");
    assert!(text.contains("fn above10"), "{text}");
    assert!(!text.contains("fn above1("), "{text}");
    assert!(!text.contains("fn above9("), "{text}");
    assert!(text.matches('…').count() >= 2, "no fold in:\n{text}");
}
