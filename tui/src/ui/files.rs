//! The files section: one row per candidate file, with its numbers.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::Frame;

use super::super::App;
use super::section;

pub fn draw(frame: &mut Frame, app: &App, area: Rect, focused: bool) {
    let rows = app
        .files
        .iter()
        .enumerate()
        .map(|(index, file)| {
            let selected = index == app.file_idx;
            let arrow = if selected { "▸" } else { " " };
            let mut line = Line::from(vec![
                Span::raw(format!("{arrow} ")),
                Span::raw(app.display(&file.path)),
                Span::raw(format!("  {}/{}", file.stats.done, file.stats.all)),
            ]);
            if selected {
                line = line.style(Style::default().add_modifier(Modifier::BOLD));
            }
            line
        })
        .collect();
    section(frame, area, "files", focused, rows);
}
