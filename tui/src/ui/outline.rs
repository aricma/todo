//! The outline: the document's headings, as a way to jump around it.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::Frame;

use super::super::App;
use super::section;

pub fn draw(frame: &mut Frame, app: &App, area: Rect, focused: bool) {
    let rows = app
        .headings()
        .iter()
        .enumerate()
        .map(|(index, heading)| {
            let indent = "  ".repeat(usize::from(heading.level.saturating_sub(1)));
            let mut line = Line::from(vec![
                Span::raw(indent),
                Span::styled(
                    format!("{} ", "#".repeat(usize::from(heading.level))),
                    Style::default().add_modifier(Modifier::DIM),
                ),
                Span::raw(heading.text.clone()),
            ]);
            if index == app.outline_idx {
                line = line.style(Style::default().add_modifier(Modifier::REVERSED));
            }
            line
        })
        .collect();
    section(frame, area, "outline", focused, rows);
}
