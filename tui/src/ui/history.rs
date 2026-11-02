//! The history section: this file's writes, newest first, as checkboxes.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::Frame;

use super::super::App;
use super::section;

pub fn draw(frame: &mut Frame, app: &App, area: Rect, focused: bool) {
    let rows = app
        .file_history()
        .iter()
        .enumerate()
        .filter_map(|(index, global)| {
            let entry = app.history.get(*global)?;
            let checkbox = if entry.applied { "x" } else { " " };
            let mut style = if entry.applied {
                Style::default()
            } else {
                Style::default().add_modifier(Modifier::DIM)
            };
            if index == app.history_idx {
                style = style.add_modifier(Modifier::REVERSED);
            }
            Some(Line::from(format!("[{checkbox}] {}", entry.label)).style(style))
        })
        .collect();
    section(frame, area, "history", focused, rows);
}
