//! Drawing: framed sections, and the two layouts a session can be in.
//!
//! Every row is built from `spans::segments`, so the colour a token gets here is
//! the colour the parser recognised.

pub mod body;
pub mod files;
pub mod hints;
pub mod history;
pub mod outline;
pub mod prompt;
pub mod status;
pub mod todos;
pub mod wrap;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;

use ratatui::widgets::{Block, BorderType, Padding, Paragraph};
use ratatui::Frame;

use super::action::Pane;
use super::App;

/// The whole screen: sections on top, one status line below.
pub fn draw(frame: &mut Frame, app: &App) {
    let [panes, status] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(2)]).areas(frame.area());
    if app.single {
        // One file: no list to choose from and nothing to compare against, so
        // every section gets the full width.
        stack(frame, app, panes, &app.sections());
    } else {
        // Two columns, so the gap between them matches the two rows a pair of
        // stacked frames leaves between sections.
        let [sidebar, view] =
            Layout::horizontal([Constraint::Percentage(38), Constraint::Percentage(62)])
                .spacing(2)
                .areas(panes);
        let panes: Vec<Pane> = app.sections().into_iter().collect();
        stack(frame, app, sidebar, &panes);
        body::draw(frame, app, view);
    }
    status::draw(frame, app, status);
}

/// How much room a section asks for, in arbitrary units that are normalised
/// over whichever sections are actually on screen.
fn weight(pane: Pane) -> u32 {
    match pane {
        Pane::Files => 34,
        Pane::Todos | Pane::View => 45,
        Pane::Outline | Pane::History => 33,
    }
}

/// Lay the given sections out down an area, one frame each.
fn stack(frame: &mut Frame, app: &App, area: Rect, panes: &[Pane]) {
    if panes.is_empty() {
        return;
    }
    let total: u32 = panes.iter().map(|pane| weight(*pane)).sum();
    let constraints: Vec<Constraint> = panes
        .iter()
        .map(|pane| Constraint::Ratio(weight(*pane), total))
        .collect();
    let areas = Layout::vertical(constraints).split(area);
    for (pane, area) in panes.iter().zip(areas.iter()) {
        let focused = app.pane == *pane;
        match pane {
            Pane::Files => files::draw(frame, app, *area, focused),
            Pane::Outline => outline::draw(frame, app, *area, focused),
            Pane::View => {}
            Pane::Todos => todos::draw(frame, app, *area, focused),
            Pane::History => history::draw(frame, app, *area, focused),
        }
    }
}

/// A section's frame: a rounded border, the title in it, and one column of
/// padding so text does not touch the border. Bright when the section has the
/// cursor, dim otherwise.
pub fn panel(title: &str, focused: bool) -> Block<'static> {
    let style = if focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().add_modifier(Modifier::DIM)
    };
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(Line::from(format!(" {title} ")))
        .border_style(style)
        .padding(Padding::horizontal(1))
}

/// Width of the `[x] ` in front of a body, and of the indent that lines a
/// wrapped todo up under it.
pub const CHECKBOX_WIDTH: usize = 4;

/// A todo's rows inside a panel: the checkbox on the first, an indent on the
/// rest, the body wrapped between words.
pub fn task_rows(task: &aricma_todo_lib::model::Task, width: usize) -> Vec<Line<'static>> {
    rows_with(task, &task.body, width)
}

/// The same rows while the todo is being edited: the body is the text typed.
pub fn editing_rows(
    task: &aricma_todo_lib::model::Task,
    prompt: &super::prompt::Prompt,
    width: usize,
) -> Vec<Line<'static>> {
    rows_with(task, prompt.text(), width)
}

fn rows_with(task: &aricma_todo_lib::model::Task, body: &str, width: usize) -> Vec<Line<'static>> {
    let body_width = width.saturating_sub(CHECKBOX_WIDTH);
    let checkbox = if task.done { "[x]" } else { "[ ]" };
    wrap::lines(body, body_width)
        .into_iter()
        .enumerate()
        .map(|(index, chunk)| {
            let prefix = if index == 0 {
                format!("{checkbox} ")
            } else {
                " ".repeat(CHECKBOX_WIDTH)
            };
            let mut spans = vec![ratatui::text::Span::raw(prefix)];
            spans.extend(prompt::line(&chunk).spans);
            Line::from(spans)
        })
        .collect()
}

/// Where the terminal cursor belongs inside an edited, wrapped todo.
pub fn cursor_at(
    inner: Rect,
    first_row: usize,
    before_the_cursor: &str,
    width: usize,
) -> (u16, u16) {
    use unicode_width::UnicodeWidthStr;

    let body_width = width.saturating_sub(CHECKBOX_WIDTH);
    let chunks = wrap::lines(before_the_cursor, body_width);
    let rows_down = chunks.len().saturating_sub(1);
    let column = UnicodeWidthStr::width(chunks.last().map_or("", String::as_str));
    let x = inner
        .x
        .saturating_add(u16::try_from(CHECKBOX_WIDTH + column).unwrap_or(u16::MAX));
    let y = inner
        .y
        .saturating_add(u16::try_from(first_row + rows_down).unwrap_or(u16::MAX));
    (
        x.min(inner.right().saturating_sub(1)),
        y.min(inner.bottom().saturating_sub(1)),
    )
}

/// Draw a section's rows inside its own frame.
pub fn section(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    focused: bool,
    rows: Vec<Line<'static>>,
) {
    let block = panel(title, focused);
    frame.render_widget(Paragraph::new(rows).block(block), area);
}

#[cfg(test)]
mod tests {
    use ratatui::style::Modifier;

    use crate::prompt::{Mode, Prompt};
    use crate::testing::{app, row, screen, screen_at, single, terminal, HEIGHT, WIDTH};

    fn texts(app: &crate::App) -> String {
        screen(app).join("\n")
    }

    #[test]
    fn every_section_is_a_framed_section_with_its_title() {
        let (_tmp, app) = app(&[(
            "TODOs.md",
            "# inbox\n\n- [ ] a\n- [x] b\n- [x] c\n\n## later\n",
        )]);
        let lines = screen(&app);
        for title in ["files", "outline", "history"] {
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
    fn the_border_is_rounded() {
        let (_tmp, app) = single("- [ ] a\n");
        let lines = screen(&app);
        assert!(lines[0].starts_with('╭'), "{:?}", lines[0]);
        assert!(lines[0].contains('╮'), "{:?}", lines[0]);
        assert!(
            lines.iter().any(|line| line.starts_with('╰')),
            "a bottom corner"
        );
    }

    #[test]
    fn the_outline_shows_headings_and_no_todos() {
        // single-file mode has no raw file view, so a heading on screen can only
        // come from the outline
        let (_tmp, app) = single("# inbox\n\n- [ ] a todo the outline must not show\n\n## later\n");
        let text = texts(&app);
        assert!(text.contains("outline"), "{text}");
        assert!(text.contains("inbox"), "{text}");
        assert!(text.contains("later"), "{text}");
        let outline_rows: Vec<&str> = text
            .lines()
            .skip_while(|line| !line.contains("outline"))
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
    fn one_heading_hides_the_outline() {
        let (_tmp, app) = app(&[("TODOs.md", "# only\n\n- [ ] a\n")]);
        assert!(!texts(&app).contains("outline"));
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
        assert!(
            !lines.iter().any(|line| line.contains("a heading")),
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
        let (_tmp, app) = single("- [x] (A) pay rent #home @sam\n");
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
        app.pane = crate::action::Pane::Todos;
        app.on_key(ratatui::crossterm::event::KeyCode::Char('e'));
        for character in "now".chars() {
            app.on_key(ratatui::crossterm::event::KeyCode::Char(character));
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
        assert!(
            lines[first + 1].contains("│     the panel"),
            "continuations are indented under the body: {}",
            lines[first + 1]
        );
        // the prose above it is still one clipped row
        let prose_row = lines.iter().position(|line| line.contains("pppp")).unwrap();
        assert!(lines[prose_row].contains('…'), "{}", lines[prose_row]);
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
        let selected = |row: usize| {
            buffer[(column, u16::try_from(row).unwrap())]
                .modifier
                .contains(Modifier::REVERSED)
        };
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

    #[test]
    fn the_selected_task_scrolls_into_view() {
        use std::fmt::Write as _;

        let mut source = String::new();
        for index in 0..60 {
            let _ = writeln!(source, "- [ ] task number {index}");
        }
        let (_tmp, mut app) = app(&[("TODOs.md", &source)]);
        app.pane = crate::action::Pane::Todos;
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
        app.pane = crate::action::Pane::Outline;
        app.outline_idx = 1;
        let lines = screen_at(&app, WIDTH, HEIGHT);
        assert!(
            lines.iter().any(|line| line.contains("## bottom")),
            "the heading the outline points at is on screen:\n{}",
            lines.join("\n")
        );
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
    fn the_prompt_colours_what_the_parser_sees() {
        let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
        app.pane = crate::action::Pane::Todos;
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
        let tag = u16::try_from(text.find("#x").unwrap()).unwrap();
        let who = u16::try_from(text.find("@y").unwrap()).unwrap();
        assert_eq!(buffer[(tag, row)].fg, super::prompt::TAG);
        assert_eq!(buffer[(who, row)].fg, super::prompt::ASSIGNEE);
    }

    #[test]
    fn history_rows_are_checkboxes_that_dim_when_reverted() {
        let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
        app.pane = crate::action::Pane::Todos;
        app.on_key(ratatui::crossterm::event::KeyCode::Char('x'));
        assert!(texts(&app).contains("[x] check · a"), "{}", texts(&app));

        app.pane = crate::action::Pane::History;
        app.on_key(ratatui::crossterm::event::KeyCode::Char('x'));
        assert!(texts(&app).contains("[ ] check · a"), "{}", texts(&app));
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
        app.pane = crate::action::Pane::Todos;
        app.on_key(ratatui::crossterm::event::KeyCode::Char('x'));
        app.pane = crate::action::Pane::History;
        app.on_key(ratatui::crossterm::event::KeyCode::Char('x'));
        insta::assert_snapshot!(screen(&app).join("\n"));
    }

    #[test]
    fn the_whole_screen_at_120x40() {
        let (_tmp, app) = app(&[(
            "TODOs.md",
            "# inbox\n\n- [ ] (B) @dave fix you #worst-bug-of-the-century\n- [ ] plain task\n- [x] done task\n\n## later\n\n- [ ] last\n",
        )]);
        insta::assert_snapshot!(screen_at(&app, 120, 40).join("\n"));
    }

    #[test]
    fn a_view_focused_screen_at_80x24() {
        let (_tmp, mut app) = app(&[(
            "TODOs.md",
            "# inbox\n\n- [ ] (A) pay rent #home @sam\n- [x] ship it\n\n## later\n",
        )]);
        app.on_key(ratatui::crossterm::event::KeyCode::Right);
        insta::assert_snapshot!(screen(&app).join("\n"));
    }

    #[test]
    fn a_wrapped_todo_at_80x24() {
        let (_tmp, app) = single(
            "- [ ] (A) @dave fix the worst bug of the century before the release #bug #infra\n\
             - [x] short one\n",
        );
        insta::assert_snapshot!(screen(&app).join("\n"));
    }

    #[test]
    fn a_single_file_at_80x24() {
        let (_tmp, app) =
            single("# inbox\n\n- [ ] (A) pay rent #home @sam\n- [x] ship it\n\n## later\n");
        insta::assert_snapshot!(screen(&app).join("\n"));
    }
}
