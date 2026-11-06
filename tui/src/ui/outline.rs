//! The index: the document's headings, as a way to jump around it.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::frame::panel;
use super::{rows as row_helpers, wrap};
use crate::helpers::as_u16;
use crate::state::App;
use crate::text;

pub fn draw(frame: &mut Frame, app: &App, area: Rect, focused: bool) {
    let width = panel(text::INDEX, focused).inner(area).width as usize;
    let mut rows: Vec<Line<'static>> = Vec::new();
    // how many rows each heading takes, so a long one can be scrolled to
    let mut heights = Vec::new();
    for (index, heading) in app.headings().iter().enumerate() {
        let before = rows.len();
        let indent = "  ".repeat(usize::from(heading.level.saturating_sub(1)));
        let hashes = format!("{} ", "#".repeat(usize::from(heading.level)));
        let pad = " ".repeat(indent.len() + hashes.len());
        let room = width.saturating_sub(pad.len());
        // a heading wraps under its own text: the hashes and the indent stay on the
        // first row, so the shape of the index is still readable
        for (line, chunk) in wrap::lines(&heading.text, room.max(super::consts::MIN_WRAP))
            .into_iter()
            .enumerate()
        {
            let mut row = if line == 0 {
                Line::from(vec![
                    Span::raw(indent.clone()),
                    Span::styled(hashes.clone(), Style::default().add_modifier(Modifier::DIM)),
                    Span::raw(chunk),
                ])
            } else {
                Line::from(vec![Span::raw(pad.clone()), Span::raw(chunk)])
            };
            if index == app.outline_idx {
                row = row_helpers::banded(row, width);
            }
            rows.push(row);
        }
        // more line height: one blank row between the headings
        if app.roomy {
            rows.push(Line::from(""));
        }
        heights.push(rows.len() - before);
    }

    // the cursor can land on a wrapped heading near the bottom, so the panel follows
    // it: the offset is counted in rows
    let height = panel(text::INDEX, focused).inner(area).height as usize;
    let (line, skip) = row_helpers::offset_for(&heights, Some(app.outline_idx), height);
    let scroll: usize = heights.iter().take(line).sum::<usize>() + skip;
    frame.render_widget(
        Paragraph::new(rows)
            .scroll((as_u16(scroll), 0))
            .block(panel(text::INDEX, focused)),
        area,
    );
    row_helpers::bar(
        frame,
        panel(text::INDEX, focused).inner(area),
        heights.iter().sum(),
        height,
        scroll,
    );
}
