//! The status bar: two rows — where you are, and what the keys do right now.

use ratatui::layout::{Alignment, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use super::hints;
use crate::state::App;
use crate::text;

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    // the version stands in the lower right corner in the text's own colour — white on
    // a dark terminal — and not dim: it is the one thing on the screen a bug report is
    // asked for, and a reader has to be able to find it. It takes its columns out of
    // the bottom row rather than being drawn over a key.
    let corner = format!("{} ", text::VERSION);
    let corner_width = UnicodeWidthStr::width(corner.as_str());
    // one column of air between the keys and the corner
    let room = usize::from(area.width).saturating_sub(corner_width + 1);
    let rows = vec![general(app), what_you_can_do(app, room)];
    frame.render_widget(Paragraph::new(rows), area);
    frame.render_widget(
        Paragraph::new(Line::from(corner)).alignment(Alignment::Right),
        last_row(area),
    );
}

/// The row the version stands on: the bottom of the status area.
const fn last_row(area: Rect) -> Rect {
    Rect {
        y: area.y + area.height.saturating_sub(1),
        height: 1,
        ..area
    }
}

/// The keys every panel shares. Where you are is not said here: the active panel
/// carries a dot, and the numbers belong to the file they count. Never a message
/// either — what went wrong is drawn under the thing it went wrong on.
fn general(app: &App) -> Line<'static> {
    Line::from(format!(" {}", hints::general_keys(app)))
}

/// What the keyboard can do right now: the modal's own keys while one is open,
/// otherwise the active panel's. On its own row so the line above never clips it.
///
/// The row stops at the corner: a key that does not fit is left off whole, because
/// half of `c copy` is not a key a reader can use.
fn what_you_can_do(app: &App, room: usize) -> Line<'static> {
    let keys = hints::modal_keys(app).unwrap_or_else(|| hints::section_keys(app));
    let mut spans = vec![Span::raw(" ")];
    let mut used = 1;
    for key in keys {
        // ` · ` between keys, and nothing before the first one
        let gap = usize::from(used > 1) * 3;
        let width = UnicodeWidthStr::width(key.as_str());
        if used + gap + width > room {
            break;
        }
        if gap > 0 {
            spans.push(Span::raw(" · "));
        }
        used += gap + width;
        spans.push(Span::raw(key));
    }
    Line::from(spans)
}
