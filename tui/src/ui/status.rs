//! The status bar: two rows — where you are, and what the keys do right now.

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::hints;
use crate::state::App;

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    // two rows always: the keys every panel shares, then the keys of whatever has
    // the keyboard right now. A question is a modal of its own, not a line here.
    let rows = vec![general(app), what_you_can_do(app)];
    frame.render_widget(Paragraph::new(rows), area);
}

/// The keys every panel shares. Where you are is not said here: the active panel
/// carries a dot, and the numbers belong to the file they count. Never a message
/// either — what went wrong is drawn under the thing it went wrong on.
fn general(app: &App) -> Line<'static> {
    Line::from(format!(" {}", hints::general_keys(app)))
}

/// What the keyboard can do right now: the modal's own keys while one is open,
/// otherwise the active panel's. On its own row so the line above never clips it.
fn what_you_can_do(app: &App) -> Line<'static> {
    let keys = hints::modal_keys(app).unwrap_or_else(|| hints::section_keys(app));
    let mut spans = vec![Span::raw(" ")];
    for (index, key) in keys.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw(" · "));
        }
        spans.push(Span::raw(key.clone()));
    }
    Line::from(spans)
}
