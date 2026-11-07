//! The todos list: a single-file session's main section, wrapping each todo the
//! same way the file view does.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::consts::message_style;
use super::frame::{counts, panel};
use super::rows::{body_width, cursor_at, offset_for, row_count, todo_rows, waiting_rows};
use crate::helpers::as_u16;
use crate::state::App;
use crate::text;

pub fn draw(frame: &mut Frame, app: &App, area: Rect, focused: bool) {
    let title = text::titled(0, text::TODOS);
    let inner = panel(&title, focused).inner(area);
    let width = inner.width as usize;
    let rows = rows(app, width);
    // the same scroll as the file view and a collection: every todo's height in rows,
    // the offset that keeps the cursor where a reader expects it, and the panel's rows
    // moved up by it. A panel that draws every row into its rect clips the list and
    // lets the cursor walk off the bottom (RFC-002's arithmetic, not a new mechanism)
    let heights = heights(app, width);
    let (line, skip) = offset_for(&heights, Some(app.task_idx), inner.height as usize);
    let scroll: usize = heights.iter().take(line).sum::<usize>() + skip;
    // the counts ride on this panel's own frame: they belong to the todos it shows
    let block = panel(&title, focused).title_bottom(counts(app, focused));
    // a single file has no file panel to put a note under, so the note goes in the
    // frame of the panel that is on screen
    let block = match app.panel_message() {
        Some(message) => block.title_bottom(Line::from(Span::styled(
            format!(" {} ", message.text),
            message_style(message.tone),
        ))),
        None => block,
    };
    frame.render_widget(
        Paragraph::new(rows)
            .scroll((as_u16(scroll), 0))
            .block(block),
        area,
    );
    super::rows::bar(
        frame,
        inner,
        heights.iter().sum(),
        inner.height as usize,
        scroll,
    );
    if let Some((x, y)) = cursor(app, area, width, scroll) {
        frame.set_cursor_position((x, y));
    }
}

/// How many rows each todo takes at this width: the same wrap the drawing uses, so the
/// scroll offset cannot drift from what is on screen.
fn heights(app: &App, width: usize) -> Vec<usize> {
    let width = width.saturating_sub(gutter(app));
    app.tasks()
        .iter()
        .map(|task| {
            let rows = row_count(task, &task.body, width);
            if app.roomy {
                rows + 1
            } else {
                rows
            }
        })
        .collect()
}

/// Every todo as one or more rows, the selected one reversed on its first row.
fn rows(app: &App, width: usize) -> Vec<Line<'static>> {
    let selected = app.task_idx;
    let gutter = gutter(app);
    let mut rows: Vec<Line<'static>> = Vec::new();
    for (index, task) in app.tasks().iter().enumerate() {
        let here = index == selected;
        let editing = app.editing().filter(|_| here);
        // the row a new todo is waiting in says so, without the file knowing
        let waiting = app.add.is_some_and(|(line, _)| line == task.line);
        // the number is a gutter, not part of the line: it stays unbanded, and the
        // todo is wrapped in what is left
        let room = width.saturating_sub(gutter);
        let cells = if waiting {
            waiting_rows(task, here, room)
        } else {
            todo_rows(task, editing, here, room)
        };
        for (at, mut row) in cells.into_iter().enumerate() {
            let number = if at == 0 {
                format!("{:>width$} ", task.line + 1, width = gutter - 1)
            } else {
                " ".repeat(gutter)
            };
            row.spans.insert(
                0,
                Span::styled(number, Style::default().add_modifier(Modifier::DIM)),
            );
            rows.push(row);
        }
        if app.roomy {
            rows.push(Line::from(""));
        }
    }
    rows
}

/// How wide the line-number column is: the digits of the last line, and a space.
fn gutter(app: &App) -> usize {
    let last = app.tasks().last().map_or(1, |task| task.line + 1);
    last.to_string().len() + 1
}

/// Where the cursor goes while a todo is edited in the list.
fn cursor(app: &App, area: Rect, width: usize, scroll: usize) -> Option<(u16, u16)> {
    let prompt = app.editing()?;
    let inner = panel(&text::titled(0, text::TODOS), true).inner(area);
    let before: String = prompt.text().chars().take(prompt.cursor()).collect();
    let tasks = app.tasks();
    // the rows are wrapped inside the line-number gutter, so the cursor is too
    let width = width.saturating_sub(gutter(app));
    // rows above this todo, minus what the panel has already scrolled past
    let above: usize = tasks
        .iter()
        .take(app.task_idx)
        .map(|task| {
            let rows = row_count(task, &task.body, width);
            if app.roomy {
                rows + 1
            } else {
                rows
            }
        })
        .sum();
    let first_row = above.saturating_sub(scroll);
    let lead = tasks
        .get(app.task_idx)
        .map_or(0, |task| width.saturating_sub(body_width(task, width)));
    let lead = lead + gutter(app);
    Some(cursor_at(inner, first_row, &before, width, lead))
}
