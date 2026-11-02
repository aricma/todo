//! The todos list: a single-file session's main section, wrapping each todo the
//! same way the file view does.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::Frame;

use super::super::App;
use super::{cursor_at, editing_rows, panel, section, task_rows};

pub fn draw(frame: &mut Frame, app: &App, area: Rect, focused: bool) {
    let inner = panel("todos", focused).inner(area);
    let width = inner.width as usize;
    let selected = app.task_idx;
    let rows = rows(app, width, selected);
    section(frame, area, "todos", focused, rows);
    if let Some((x, y)) = cursor(app, area, width) {
        frame.set_cursor_position((x, y));
    }
}

/// Every todo as one or more rows, the selected one reversed on its first row.
fn rows(app: &App, width: usize, selected: usize) -> Vec<Line<'static>> {
    let mut rows = Vec::new();
    for (index, task) in app.tasks().iter().enumerate() {
        let rendered = match app.editing().filter(|_| index == selected) {
            Some(prompt) => editing_rows(task, prompt, width),
            None => task_rows(task, width),
        };
        for row in rendered {
            // the highlight covers the whole todo, continuation rows too
            rows.push(if index == selected {
                row.style(Style::default().add_modifier(Modifier::REVERSED))
            } else {
                row
            });
        }
    }
    rows
}

/// Where the cursor goes while a todo is edited in the list.
fn cursor(app: &App, area: Rect, width: usize) -> Option<(u16, u16)> {
    let prompt = app.editing()?;
    let inner = panel("todos", true).inner(area);
    let before: String = prompt.text().chars().take(prompt.cursor()).collect();
    let body_width = width.saturating_sub(super::CHECKBOX_WIDTH);
    let first_row: usize = app
        .tasks()
        .iter()
        .take(app.task_idx)
        .map(|task| super::wrap::lines(&task.body, body_width).len())
        .sum();
    Some(cursor_at(inner, first_row, &before, width))
}
