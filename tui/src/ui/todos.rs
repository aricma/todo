//! The todos list: a single-file session's main section, wrapping each todo the
//! same way the file view does.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::consts::message_style;
use super::frame::{counts, panel};
use super::rows::{body_width, cursor_at, row_count, todo_rows};
use crate::state::App;
use crate::text;

pub fn draw(frame: &mut Frame, app: &App, area: Rect, focused: bool) {
    let width = panel(text::TODOS, focused).inner(area).width as usize;
    let rows = rows(app, width);
    // the counts ride on this panel's own frame: they belong to the todos it shows
    let block = panel(text::TODOS, focused).title_bottom(counts(app, focused));
    // a single file has no file panel to put a note under, so the note goes in the
    // frame of the panel that is on screen
    let block = match app.panel_message() {
        Some(message) => block.title_bottom(Line::from(Span::styled(
            format!(" {} ", message.text),
            message_style(message.tone),
        ))),
        None => block,
    };
    frame.render_widget(Paragraph::new(rows).block(block), area);
    if let Some((x, y)) = cursor(app, area, width) {
        frame.set_cursor_position((x, y));
    }
}

/// Every todo as one or more rows, the selected one reversed on its first row.
fn rows(app: &App, width: usize) -> Vec<Line<'static>> {
    let selected = app.task_idx;
    let gutter = gutter(app);
    let mut rows: Vec<Line<'static>> = Vec::new();
    for (index, task) in app.tasks().iter().enumerate() {
        let here = index == selected;
        let editing = app.editing().filter(|_| here);
        // the number is a gutter, not part of the line: it stays unbanded, and the
        // todo is wrapped in what is left
        for (at, mut row) in todo_rows(task, editing, here, width.saturating_sub(gutter))
            .into_iter()
            .enumerate()
        {
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
fn cursor(app: &App, area: Rect, width: usize) -> Option<(u16, u16)> {
    let prompt = app.editing()?;
    let inner = panel(text::TODOS, true).inner(area);
    let before: String = prompt.text().chars().take(prompt.cursor()).collect();
    let tasks = app.tasks();
    // the rows are wrapped inside the line-number gutter, so the cursor is too
    let width = width.saturating_sub(gutter(app));
    let first_row: usize = tasks
        .iter()
        .take(app.task_idx)
        .map(|task| row_count(task, &task.body, width))
        .sum();
    let lead = tasks
        .get(app.task_idx)
        .map_or(0, |task| width.saturating_sub(body_width(task, width)));
    let lead = lead + gutter(app);
    Some(cursor_at(inner, first_row, &before, width, lead))
}
