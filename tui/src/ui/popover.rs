//! The search popover: a query line and the todos it found, over the screen.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph, Wrap};
use ratatui::Frame;

use super::consts::{message_prefix, message_style};
use super::consts::{
    ADD_MAX_WIDTH, KEYS_COLUMN_GAP, KEYS_MAX_WIDTH, KEYS_PAD_COLS, KEYS_PAD_ROWS, MIN_WRAP, PAD,
    POPOVER_BORDER_ROWS, POPOVER_HALF, POPOVER_MIN_WIDTH, POPOVER_THIRD, SEARCH_HEIGHT_DEN,
    SEARCH_HEIGHT_NUM, SEARCH_MIN_HEIGHT, SEARCH_MIN_WIDTH, SEARCH_WIDTH_DEN, SEARCH_WIDTH_NUM,
    TEXT_ROWS,
};
use super::cursor_in;

/// The rule under the typed text, and the rows it takes.
const RULE: &str = "─";
const RULE_ROWS: u16 = 1;
use super::frame::{bottom_right, floating};
use super::hints::Where;
use super::{hints, prompt, rows, wrap};
use crate::helpers::as_u16;
use crate::state::action::Pane;
use crate::state::confirm::Confirm;
use crate::state::create::FILE_NAME;
use crate::state::message;
use crate::state::prompt::{Mode, Prompt};
use crate::state::App;
use crate::text;

/// How wide and tall the popover wants to be, inside the screen it is given.
///
/// The height is what the query and its hits need, not a share of the screen: a
/// search that found one todo should not cover two thirds of the terminal to say so.
/// The tall limit is where the list starts scrolling instead of growing.
fn area(screen: Rect, rows: u16) -> Rect {
    let width = (screen.width.saturating_mul(SEARCH_WIDTH_NUM) / SEARCH_WIDTH_DEN)
        .max(SEARCH_MIN_WIDTH)
        .min(screen.width);
    let height = rows
        .saturating_add(POPOVER_BORDER_ROWS)
        .max(SEARCH_MIN_HEIGHT)
        .min(screen.height.saturating_mul(SEARCH_HEIGHT_NUM) / SEARCH_HEIGHT_DEN)
        .min(screen.height);
    Rect {
        x: screen.x + (screen.width - width) / 2,
        y: screen.y + (screen.height - height) / 3,
        width,
        height,
    }
}

pub fn draw(frame: &mut Frame, app: &App) {
    backdrop(frame, app);
    draw_add(frame, app);
    draw_create(frame, app);
    draw_help(frame, app);
    draw_conflict(frame, app);
    draw_confirm(frame, app);
    if app.mode != Mode::Searching {
        return;
    }
    let area = area(frame.area(), app.search.height());
    // the popover floats: what is under it must not show through
    frame.render_widget(Clear, area);
    // how many todos matched, in the title: the list is a window on them, and the
    // window cannot say how much of the list it is showing
    let title = match app.search.hits.len() {
        0 => text::SEARCH.to_owned(),
        count => format!("{} · {}", text::SEARCH, text::matches(count)),
    };
    // the keys the popover owns, where every other modal keeps them; `enter` is
    // only in there when there is a hit to go to
    let can_jump = app.search.current().is_some();
    let block = floating(&title).title_bottom(bottom_right(text::search_keys(can_jump)));
    let inner = block.inner(area);
    frame.render_widget(Paragraph::new(rows(app, inner)).block(block), area);

    // the cursor sits in the query, one row down inside the frame
    let before: String = app
        .search
        .query
        .text()
        .chars()
        .take(app.search.query.cursor())
        .collect();
    cursor_in(frame, inner, &before, PAD);
}

/// Everything behind a modal is dimmed, so the panel that has the keyboard is the
/// one the eye lands on: a floating frame over a screen of the same brightness reads
/// as part of it, not as a question. Dim is a modifier a terminal may ignore — the
/// screen then looks exactly as it did, and nothing is broken, which is the reason
/// it is a modifier and not a colour painted over the text.
fn backdrop(frame: &mut Frame, app: &App) {
    if !app.floating() {
        return;
    }
    let screen = frame.area();
    frame
        .buffer_mut()
        .set_style(screen, Style::default().add_modifier(Modifier::DIM));
}

/// The conflict question: the file changed under a write of ours, and only a person
/// can say which version is the one they meant.
fn draw_conflict(frame: &mut Frame, app: &App) {
    if app.mode != Mode::Conflicted {
        return;
    }
    let screen = frame.area();
    let width = (screen.width / POPOVER_HALF)
        .clamp(POPOVER_MIN_WIDTH, ADD_MAX_WIDTH)
        .min(screen.width);
    let path = app.display(&app.current().path);
    let rows = wrap::lines(&format!("{path} changed on disk"), inner_width(width));
    let height = POPOVER_BORDER_ROWS + as_u16(rows.len().max(1)) + RULE_ROWS;
    let area = Rect {
        x: screen.x + (screen.width - width) / 2,
        y: screen.y + screen.height / POPOVER_THIRD,
        width,
        height: height.min(screen.height),
    };
    frame.render_widget(Clear, area);
    let block = floating(text::CONFLICT).title_bottom(bottom_right(text::CONFLICT_KEYS));
    let inner = block.inner(area);
    let mut lines: Vec<Line<'static>> = rows.into_iter().map(Line::from).collect();
    lines.push(Line::from(Span::styled(
        RULE.repeat(inner.width as usize),
        Style::default().add_modifier(Modifier::DIM),
    )));
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

/// The width inside a frame of this width: one column of padding each side, and the
/// two borders.
fn inner_width(width: u16) -> usize {
    usize::from(width.saturating_sub(2 * PAD + 2))
}

/// The delete question, as a modal: a line in the status bar is easy to miss, and
/// this is the last moment before something is gone.
fn draw_confirm(frame: &mut Frame, app: &App) {
    let Mode::Confirming(confirm) = &app.mode else {
        return;
    };
    let what = match confirm {
        Confirm::Todo => app.current_task().map(|task| task.body).unwrap_or_default(),
        Confirm::File => app.display(&app.current().path),
    };
    let (title, keys) = match confirm {
        Confirm::Todo => (text::DELETE_TODO, text::CANCEL_DELETE_TODO),
        Confirm::File => (text::DELETE_FILE, text::CANCEL_DELETE_FILE),
    };
    let screen = frame.area();
    let width = (screen.width / POPOVER_HALF)
        .max(POPOVER_MIN_WIDTH)
        .min(screen.width);
    let room = usize::from(width.saturating_sub(2 * PAD + 2)).max(MIN_WRAP);
    let rows: Vec<Line<'static>> = wrap::lines(&what, room)
        .into_iter()
        .map(|chunk| prompt::line(&chunk))
        .collect();
    let height = as_u16(rows.len()).saturating_add(POPOVER_BORDER_ROWS);
    let area = Rect {
        x: screen.x + (screen.width.saturating_sub(width)) / 2,
        y: screen.y + screen.height / POPOVER_THIRD,
        width,
        height: height.min(screen.height),
    };
    frame.render_widget(Clear, area);
    let block = floating(title).title_bottom(bottom_right(keys));
    frame.render_widget(Paragraph::new(rows).block(block), area);
}

/// The keys page: every binding, grouped by the panel it works in, the panel the
/// cursor is in first. Three columns, because one column of every binding is taller
/// than a terminal.
fn draw_help(frame: &mut Frame, app: &App) {
    if app.mode != Mode::Help {
        return;
    }
    let screen = frame.area();
    let columns = help_columns(app);
    // the page is as wide as its three columns need: a box of empty space tells a
    // reader nothing, and a column narrower than its longest row wraps the words onto
    // a line of their own
    let widths: Vec<u16> = columns.iter().map(|column| column_width(column)).collect();
    let needed = widths.iter().sum::<u16>()
        + as_u16(widths.len().saturating_sub(1)) * KEYS_COLUMN_GAP
        + POPOVER_BORDER_ROWS
        + 2 * (PAD + KEYS_PAD_COLS);
    let width = needed.min(screen.width).min(KEYS_MAX_WIDTH);
    let height = as_u16(columns.iter().map(Vec::len).max().unwrap_or_default())
        .saturating_add(POPOVER_BORDER_ROWS)
        .saturating_add(KEYS_PAD_ROWS);
    let area = Rect {
        x: screen.x + (screen.width.saturating_sub(width)) / 2,
        y: screen.y + (screen.height.saturating_sub(height)) / 2,
        width,
        height: height.min(screen.height),
    };
    frame.render_widget(Clear, area);
    // the border says the last key that arrived: "this key does nothing" is
    // usually the terminal, and this is the only way to see what it sent
    let seen = app.last_key.as_ref().map_or_else(
        || text::LAST_KEY.to_owned(),
        |key| format!(" last key: {} ", hints::key_name(key)),
    );
    let block = floating(text::KEYS)
        .title_bottom(Line::from(seen))
        .title_bottom(bottom_right(text::CLOSE_KEYS));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // a page of keys is a wall of text without air: one row above the columns, one
    // more column inside each border than a panel takes, and a real gap between the
    // columns rather than a single cell that reads as part of the words either side
    let inner = Rect {
        x: inner.x.saturating_add(KEYS_PAD_COLS),
        y: inner.y.saturating_add(KEYS_PAD_ROWS),
        width: inner.width.saturating_sub(2 * KEYS_PAD_COLS),
        height: inner.height.saturating_sub(KEYS_PAD_ROWS),
    };
    let areas = Layout::horizontal(widths.iter().map(|width| Constraint::Length(*width)))
        .spacing(KEYS_COLUMN_GAP)
        .split(inner);
    for (column, area) in columns.iter().zip(areas.iter()) {
        frame.render_widget(
            Paragraph::new(column.clone()).wrap(Wrap { trim: false }),
            *area,
        );
    }
}

/// How wide a column has to be, in cells: its longest row.
fn column_width(column: &[Line<'static>]) -> u16 {
    as_u16(column.iter().map(Line::width).max().unwrap_or_default())
}

/// The page in three columns: the keys every panel shares and the ones a field takes,
/// the panels that choose where you are, and the panels that edit a todo.
///
/// Every row comes from the one table, so a key cannot be listed here and missing
/// from the bar, or the other way round.
fn help_columns(app: &App) -> [Vec<Line<'static>>; 3] {
    // the panel the cursor is in says so, the way the panels themselves do
    let mark = |pane: Pane| {
        let here = app.pane == pane
            || (pane == Pane::Todos && app.pane == Pane::View)
            || (pane == Pane::Collection && app.pane == Pane::Collection);
        if here {
            "● "
        } else {
            ""
        }
    };
    let column = |pane: Pane| group(&format!("{}{}", mark(pane), pane.name()), &bindings(pane));

    // a collection shares almost every key with the todo panels, so only its own row
    // is listed: the note says what is missing, which is shorter than the eight rows
    // that would say the same thing
    let mut collection = group(
        &format!("{}{}", mark(Pane::Collection), text::COLLECTION),
        &hints::alone(Where::COLLECTION),
    );
    for note in text::COLLECTION_KEYS_NOTE {
        collection.insert(
            collection.len().saturating_sub(1),
            Line::from(Span::styled(
                note.to_string(),
                Style::default().add_modifier(Modifier::DIM),
            )),
        );
    }

    let mut columns = [
        [
            group(text::GENERAL, &hints::bindings(&[Where::ANY])),
            group(text::TYPING, &additions(&[Where::TYPING])),
        ]
        .concat(),
        [
            column(Pane::Files),
            column(Pane::Outline),
            column(Pane::History),
        ]
        .concat(),
        [column(Pane::Todos), collection].concat(),
    ];
    for column in &mut columns {
        column.pop();
    }
    columns
}

/// What one panel adds to the keys every panel shares: the rows of the table that
/// work in it, and only those — the shared ones are already in the first column.
fn bindings(pane: Pane) -> Vec<&'static hints::Binding> {
    let panes = match pane {
        Pane::Files => vec![Where::FILES, Where::FILE_ROW],
        Pane::Outline => vec![Where::OUTLINE],
        Pane::View | Pane::Todos => vec![Where::TODO],
        Pane::Collection => vec![Where::COLLECTION],
        Pane::History => vec![Where::HISTORY],
    };
    additions(&panes)
}

/// The rows that work in these places and are not the keys every panel shares: those
/// are in the first column, once.
fn additions(panes: &[Where]) -> Vec<&'static hints::Binding> {
    hints::bindings(panes)
        .into_iter()
        .filter(|binding| binding.panes != Where::ANY)
        .collect()
}

/// One titled group of bindings: the title, then the keys padded into a column of
/// their own, then a blank row.
fn group(title: &str, bindings: &[&hints::Binding]) -> Vec<Line<'static>> {
    // in cells, not bytes: an arrow is one cell and three bytes, and padding by
    // bytes would leave a hole in every row after it
    let width = bindings
        .iter()
        .map(|b| Span::raw(b.keys).width())
        .max()
        .unwrap_or(0);
    let mut rows = vec![Line::from(Span::styled(
        title.to_owned(),
        Style::default().add_modifier(Modifier::BOLD),
    ))];
    rows.extend(bindings.iter().map(|b| {
        // the keys padded into a column of their own, then two cells before the words:
        // one reads as part of a key that starts with an arrow
        let pad = " ".repeat(width.saturating_sub(Span::raw(b.keys).width()) + 2);
        Line::from(vec![
            Span::raw(b.keys),
            Span::raw(pad),
            Span::raw(b.does),
        ])
    }));
    rows.push(Line::from(""));
    rows
}

/// The field a new todo file is typed in: the folder, what that folder offers, and
/// why the path is refused. The file itself is always `TODOs.md`.
pub fn draw_create(frame: &mut Frame, app: &App) {
    let Mode::Creating(create) = &app.mode else {
        return;
    };
    let screen = frame.area();
    let width = (screen.width / POPOVER_HALF)
        .max(POPOVER_MIN_WIDTH)
        .min(screen.width);
    // the typed folder, whatever it offers, and why it is refused
    let extra = as_u16(create.suggestions.len()) + u16::from(create.refusal.is_some());
    let height = POPOVER_BORDER_ROWS + TEXT_ROWS + extra;
    let area = Rect {
        x: screen.x + (screen.width - width) / 2,
        y: screen.y + screen.height / POPOVER_THIRD,
        width,
        height: height.min(screen.height),
    };
    frame.render_widget(Clear, area);
    let block = floating(text::NEW_FILE).title_bottom(bottom_right(text::CANCEL_CREATE));
    let inner = block.inner(area);

    // the whole path, in full: the parts that cannot change are grey, what is typed
    // is not, and the cursor only ever lives in the middle
    let typed = create.prompt.text();
    let suffix = if typed.is_empty() || typed.ends_with('/') {
        FILE_NAME
    } else {
        ""
    };
    let mut first = vec![
        Span::styled("./", Style::default().add_modifier(Modifier::DIM)),
        Span::raw(typed.to_owned()),
    ];
    if suffix.is_empty() {
        first.push(Span::raw("/"));
        first.push(Span::styled(
            FILE_NAME,
            Style::default().add_modifier(Modifier::DIM),
        ));
    } else {
        first.push(Span::styled(
            suffix,
            Style::default().add_modifier(Modifier::DIM),
        ));
    }
    let mut rows = vec![Line::from(first)];
    for (index, suggestion) in create.suggestions.iter().enumerate() {
        let mut line = Line::from(Span::styled(
            format!("  {suggestion}"),
            Style::default().add_modifier(Modifier::DIM),
        ));
        if create.selected == Some(index) {
            // marked with the arrows: typing on takes it
            line = super::rows::banded(line, inner.width as usize);
        }
        rows.push(line);
    }
    if let Some(refusal) = &create.refusal {
        // the field refuses its own path, in the warning colour: nothing failed,
        // the rule is just being explained
        rows.push(Line::from(Span::styled(
            refusal.clone(),
            message_style(message::Tone::Warning),
        )));
    }
    frame.render_widget(Paragraph::new(rows).block(block), area);

    // the cursor sits in the typed part, after the grey `./`
    let before: String = create
        .prompt
        .text()
        .chars()
        .take(create.prompt.cursor())
        .collect();
    cursor_in(frame, inner, &before, PAD);
}

/// The field a new todo or a new heading is typed in.
///
/// It floats, because what is being made does not exist yet: a todo's empty row is in
/// the file and highlighted, which says *where* it goes, and the field says *what* —
/// typed into a row that is only a checkbox and a title waiting, there would be nothing
/// on screen telling the person that a line is open for them.
pub fn draw_add(frame: &mut Frame, app: &App) {
    let (prompt, title) = match &app.mode {
        Mode::Adding(prompt) => (prompt, text::NEW_TODO),
        Mode::AddingHeading(prompt) => (prompt, text::NEW_HEADING),
        _ => return,
    };
    let screen = frame.area();
    // half the screen, like the other popovers, and never wider than a sentence:
    // what does not fit wraps onto the next row
    let width = (screen.width / POPOVER_HALF)
        .clamp(POPOVER_MIN_WIDTH, ADD_MAX_WIDTH)
        .min(screen.width);
    let panel = app.panel_message();
    let area = Rect {
        x: screen.x + (screen.width - width) / 2,
        y: screen.y + screen.height / POPOVER_THIRD,
        width,
        // measured below, once the frame has taken its borders off the width
        height: 1,
    };
    // the width is the frame's business, so the height is measured after the
    // borders are taken off
    let block = floating(title).title_bottom(bottom_right(text::CANCEL_ADD));
    let inner = block.inner(Rect {
        height: u16::MAX,
        ..area
    });
    let rows_wanted = wrap::lines(prompt.text(), inner.width as usize).len();
    // two borders, the text, and one row more when the field has to say why it
    // refused; the keys live in the frame, so they cost no row
    let height = POPOVER_BORDER_ROWS + as_u16(rows_wanted.max(1)) + u16::from(panel.is_some());
    let area = Rect {
        height: height.min(screen.height),
        ..area
    };
    frame.render_widget(Clear, area);
    let inner = block.inner(area);
    let rows: Vec<Line<'static>> = wrap::lines(prompt.text(), inner.width as usize)
        .into_iter()
        .map(|chunk| prompt::line(&chunk))
        .collect();
    let mut rows = rows;
    if let Some(message) = panel {
        rows.push(Line::from(Span::styled(
            format!("{}{}", message_prefix(message.tone), message.text),
            message_style(message.tone),
        )));
    }
    frame.render_widget(Paragraph::new(rows).block(block), area);

    // the cursor follows the text into the second and third row: a field that wraps
    // and then keeps its cursor on the first line looks broken
    let before: String = prompt.text().chars().take(prompt.cursor()).collect();
    let (x, y) = super::rows::cursor_at(inner, 0, &before, inner.width as usize, 0);
    frame.set_cursor_position((x, y));
}

/// The query line, then one row per result.
fn rows(app: &App, inner: Rect) -> Vec<Line<'static>> {
    let mut rows = vec![query_line(&app.search.query)];
    // what the search has to say about its own results: nothing found, or why it
    // could not run
    if let Some(message) = &app.search.message {
        rows.push(Line::from(Span::styled(
            format!("{}{}", message_prefix(message.tone), message.text),
            message_style(message.tone),
        )));
    }
    let room = inner.height.saturating_sub(2) as usize;
    // the list scrolls to the selection like every other list: a hit list is as
    // long as the tree holds, and walking off the bottom of a popover with no way
    // to see what is selected is losing the cursor
    let heights = vec![1; app.search.hits.len()];
    let (first, _) = rows::offset_for(&heights, Some(app.search.selected), room.max(1));
    for (index, hit) in app.search.hits.iter().enumerate().skip(first).take(room) {
        // the file and the line, so two todos with the same text are told apart
        let mut spans = vec![Span::raw(format!(
            "{}:{}  ",
            app.display(&hit.path),
            hit.line + 1
        ))];
        spans.extend(prompt::line(&hit.body).spans);
        let mut line = Line::from(spans);
        if index == app.search.selected {
            line = super::rows::banded(line, inner.width as usize);
        } else {
            line = line.style(Style::default().add_modifier(Modifier::DIM));
        }
        rows.push(line);
    }
    rows
}

fn query_line(query: &Prompt) -> Line<'static> {
    let mut spans = vec![Span::raw("> ")];
    spans.extend(prompt::line(query.text()).spans);
    Line::from(spans)
}

