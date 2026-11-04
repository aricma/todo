//! Frames: the rounded box a section lives in, and the titles on its border.

use ratatui::layout::Alignment;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Padding};

use super::consts::{ACCENT, PAD};
use crate::state::App;

/// A section's frame: a rounded border, the title in it, and one column of
/// padding so text does not touch the border. Bright when the section has the
/// cursor, dim otherwise.
pub fn panel(title: &str, focused: bool) -> Block<'static> {
    framed(title, focused, focused)
}

/// A title for the right end of a frame's bottom border: the counts, the keys of a
/// field. Right, because the left of the border belongs to the title above it.
pub fn bottom_right(text: impl Into<String>) -> Line<'static> {
    Line::from(text.into()).alignment(Alignment::Right)
}

/// A field over the screen: bright, but without the dot — it has the focus by
/// definition, so a mark saying so is noise.
pub fn floating(title: &str) -> Block<'static> {
    framed(title, true, false)
}

/// `mark` is the dot in front of the title that says where the cursor is. A colour
/// alone is easy to miss on a screen of framed panels; the dot is not.
fn framed(title: &str, focused: bool, mark: bool) -> Block<'static> {
    let style = if focused {
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().add_modifier(Modifier::DIM)
    };
    let title = if mark {
        format!(" ● {title} ")
    } else {
        format!(" {title} ")
    };
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(Line::from(title))
        .border_style(style)
        .padding(Padding::horizontal(PAD))
}

/// The numbers a panel of todos carries on its bottom border: `3/7`, and the distinct
/// tags and people the file mentions, in the colours the rows give them.
///
/// Right-aligned with a space either side, because the left of the border belongs to
/// the title above it. They are drawn in the border's own style: a number that sits
/// *on* a border and does not brighten with it reads as part of the panel behind it.
pub fn counts(app: &App, focused: bool) -> Line<'static> {
    // a collection counts its own todos; a file counts the document that is open
    let stats = if app.viewing().is_some() {
        app.collection_stats()
    } else {
        app.current().stats
    };
    // the border's style, so the numbers read as the border's own writing
    let base = if focused {
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().add_modifier(Modifier::DIM)
    };
    let mut spans = vec![
        Span::styled(" ", base),
        Span::styled(format!("{}/{}", stats.done, stats.all), base),
    ];
    // a collection says nothing about tags and people: it *is* one
    if stats.tags > 0 {
        spans.push(Span::styled(
            format!("  #{}", stats.tags),
            Style::default().fg(super::prompt::TAG),
        ));
    }
    if stats.assignees > 0 {
        spans.push(Span::styled(
            format!("  @{}", stats.assignees),
            Style::default().fg(super::prompt::ASSIGNEE),
        ));
    }
    spans.push(Span::styled(" ", base));
    Line::from(spans).alignment(Alignment::Right)
}

/// What a panel's bottom border says: the message it has for now, or its numbers.
///
/// A panel with no list of its own — the file view, a collection — has nowhere under
/// its rows to put a message, and a message that shifted the document would be worse
/// than the thing it reports, so it takes the footer for the one keystroke it lasts.
pub fn footer(app: &App, focused: bool) -> Line<'static> {
    match app.panel_message() {
        Some(message) => Line::from(vec![
            Span::raw(" "),
            Span::styled(
                message.text.clone(),
                super::consts::message_style(message.tone),
            ),
            Span::raw(" "),
        ])
        .alignment(Alignment::Right),
        None => counts(app, focused),
    }
}
