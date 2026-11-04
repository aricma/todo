//! The whole screen: which sections go where, and how much room each one gets.

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::Frame;

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::consts::{
    COLUMN_GAP, FILES_WEIGHT, HEADER_ROWS, SIDEBAR_PERCENT, SIDE_WEIGHT, STATUS_ROWS, TODOS_WEIGHT,
    VIEW_PERCENT,
};
use super::{body, collection, files, history, outline, popover, status, todos};
use crate::helpers::{shorten, truncate};
use crate::state::{App, Pane};
use crate::text;
use unicode_width::UnicodeWidthStr;

/// The whole screen: what this is and where it is, the sections, one status line.
pub fn draw(frame: &mut Frame, app: &App) {
    let [header, panes, status] = Layout::vertical([
        Constraint::Length(HEADER_ROWS),
        Constraint::Min(1),
        Constraint::Length(STATUS_ROWS),
    ])
    .areas(frame.area());
    header_line(frame, app, header);
    if app.single {
        // One file: no list to choose from and nothing to compare against, so
        // every section gets the full width.
        stack(frame, app, panes, &app.sections());
    } else {
        // Two columns, so the gap between them matches the two rows a pair of
        // stacked frames leaves between sections.
        let [sidebar, view] = Layout::horizontal([
            Constraint::Percentage(SIDEBAR_PERCENT),
            Constraint::Percentage(VIEW_PERCENT),
        ])
        .spacing(COLUMN_GAP)
        .areas(panes);
        let panes: Vec<Pane> = app.sections().into_iter().collect();
        stack(frame, app, sidebar, &panes);
        // the right-hand side is the open file, or the collection that was opened
        // from the file panel
        if app.viewing().is_some() {
            collection::draw(frame, app, view, app.pane == Pane::Collection);
        } else {
            body::draw(frame, app, view);
        }
    }
    status::draw(frame, app, status);
    popover::draw(frame, app);
}

/// `✅ aricma-lazytodo · /where/it/was/opened`: the one line that says what is on
/// screen and which tree it belongs to — two sessions on two checkouts look identical
/// otherwise.
fn header_line(frame: &mut Frame, app: &App, area: Rect) {
    // one column of air either side, like the status bar has: a header that starts
    // in the terminal's first cell reads as part of its frame
    const MARGIN: usize = 1;

    let width = usize::from(area.width);
    // a success says what worked, in the one place that is not part of any panel:
    // nothing on screen has to move over for it, and the next key takes it away.
    // It never takes more than half the row, so the tree stays readable
    let success = app
        .success_message()
        .map(|message| truncate(&message.text, width / 2));
    let taken = success
        .as_ref()
        .map_or(0, |text| UnicodeWidthStr::width(text.as_str()) + MARGIN + 1);

    // the tail of a long path says more than its head, so it is cut from the front.
    // `✅` is two cells wide, so the room is measured, not counted.
    let used = MARGIN + Span::raw(text::APP).width() + Span::raw(" · ").width();
    let room = width.saturating_sub(used + taken);
    let where_it_is = shorten(&app.root.display().to_string(), room);
    let line = Line::from(vec![
        Span::raw(" ".repeat(MARGIN)),
        Span::styled(text::APP, Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(" · ", Style::default().add_modifier(Modifier::DIM)),
        Span::styled(where_it_is, Style::default().add_modifier(Modifier::DIM)),
    ]);
    frame.render_widget(Paragraph::new(line), area);

    if let Some(text) = success {
        let right = Paragraph::new(Line::from(Span::styled(
            format!("{text}{}", " ".repeat(MARGIN)),
            super::consts::message_style(crate::state::message::Tone::Success),
        )))
        .alignment(Alignment::Right);
        frame.render_widget(right, area);
    }
}

/// How much room a section asks for, in arbitrary units that are normalised
/// over whichever sections are actually on screen.
fn weight(pane: Pane) -> u32 {
    match pane {
        Pane::Files => FILES_WEIGHT,
        // the collection is the whole right-hand side, like the file view
        Pane::Todos | Pane::View | Pane::Collection => TODOS_WEIGHT,
        Pane::Outline | Pane::History => SIDE_WEIGHT,
    }
}

/// Lay the given sections out down an area, one frame each.
fn stack(frame: &mut Frame, app: &App, area: Rect, panes: &[Pane]) {
    if panes.is_empty() {
        return;
    }
    let total: u32 = panes.iter().map(|pane| weight(*pane)).sum();
    let constraints: Vec<Constraint> = panes
        .iter()
        .map(|pane| Constraint::Ratio(weight(*pane), total))
        .collect();
    let areas = Layout::vertical(constraints).split(area);
    for (pane, area) in panes.iter().zip(areas.iter()) {
        let focused = app.pane == *pane;
        match pane {
            Pane::Files => files::draw(frame, app, *area, focused),
            Pane::Outline => outline::draw(frame, app, *area, focused),
            Pane::View | Pane::Collection => {}
            Pane::Todos => todos::draw(frame, app, *area, focused),
            Pane::History => history::draw(frame, app, *area, focused),
        }
    }
}
