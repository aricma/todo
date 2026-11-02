//! The status bar: two rows — where you are, and what the keys do right now.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::super::prompt::Mode;
use super::super::App;
use super::{hints, prompt};

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    match &app.mode {
        // a new todo has no row to be typed into yet, so it is typed here
        Mode::Adding(text) => typing(frame, area, "new todo", text.text(), text.cursor()),
        // an edited todo is typed on its own row, so the bar stays quiet
        Mode::Editing(_) | Mode::Normal => {
            let rows = vec![where_you_are(app), what_you_can_do(app)];
            frame.render_widget(Paragraph::new(rows), area);
        }
        Mode::Confirming => {
            let what = app.current_task().map(|task| task.body).unwrap_or_default();
            let line = Line::from(format!("delete {what}? y/n"));
            frame.render_widget(
                Paragraph::new(line).style(Style::default().add_modifier(Modifier::REVERSED)),
                area,
            );
        }
    }
}

/// The numbers, the active section, and a message when there is one.
fn where_you_are(app: &App) -> Line<'static> {
    let stats = app.current().stats;
    let place = hints::where_and_navigation(app);
    let line = match &app.message {
        Some(message) => format!(" {}/{}  {message}", stats.done, stats.all),
        None => format!(" {}/{}  {place}", stats.done, stats.all),
    };
    Line::from(line)
}

/// What this section adds, on its own row so the line above never clips it.
fn what_you_can_do(app: &App) -> Line<'static> {
    let keys = hints::section_keys(app.pane);
    let mut spans = vec![Span::raw(" ")];
    for (index, key) in keys.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw(" · "));
        }
        spans.push(Span::raw((*key).to_owned()));
    }
    Line::from(spans)
}

/// A prompt row: the label, the coloured text as typed, and the cursor.
fn typing(frame: &mut Frame, area: Rect, label: &str, text: &str, cursor: usize) {
    let mut spans = vec![Span::raw(format!("{label}: "))];
    spans.extend(prompt::line(text).spans);
    frame.render_widget(Paragraph::new(Line::from(spans)), area);

    let before: String = text.chars().take(cursor).collect();
    let x = area
        .x
        .saturating_add(u16::try_from(label.chars().count()).unwrap_or(u16::MAX))
        .saturating_add(2)
        .saturating_add(display_width(&before));
    frame.set_cursor_position((x.min(area.right().saturating_sub(1)), area.y));
}

fn display_width(text: &str) -> u16 {
    use unicode_width::UnicodeWidthStr;
    u16::try_from(UnicodeWidthStr::width(text)).unwrap_or(u16::MAX)
}
