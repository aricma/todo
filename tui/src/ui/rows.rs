//! Rows: a todo as one or more lines, the cursor inside one being edited, and
//! where the viewport starts.

use aricma_todo_lib::model::{Segment, Task};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use super::consts::{SCROLL_MARGIN_DEN, SCROLL_MARGIN_NUM};
use super::prompt;
use super::wrap;
use crate::helpers::as_u16;

/// The columns a band may fill: the panel's text, less the one the bar lives in.
///
/// A band stops *before* the bar rather than underneath it, so the bar needs no
/// background of its own: it is drawn on the panel's ground and never has to know what
/// it covers. One column is also nothing to lose — the right edge of a panel is air.
pub fn band_end(width: usize) -> usize {
    width.saturating_sub(1)
}

/// Where the viewport starts: a document line, and how many of its first rows to
/// skip.
///
/// The skip is what keeps the selection still. Todos wrap to one or three rows,
/// so an offset that could only start on a line boundary would land the
/// checkmark anywhere between the margin and the bottom of whatever line
/// happened to sit there — the viewport would jump by a todo's height on every
/// move. Counting in rows instead pins the checkmark to the margin's last row,
/// whatever the neighbours look like.
///
/// The top third is a margin the selection moves through without the document
/// moving at all; past it the viewport follows. At the end the document clamps
/// to its last full page, so the selection walks down into the bottom third
/// instead of the panel filling with empty rows — how far depends on what
/// follows the last todo, since trailing prose keeps it higher.
pub fn offset_for(heights: &[usize], selected: Option<usize>, height: usize) -> (usize, usize) {
    let (Some(selected), true) = (selected, height > 0) else {
        return (0, 0);
    };
    let selected = selected.min(heights.len().saturating_sub(1));
    let margin = height * SCROLL_MARGIN_NUM / SCROLL_MARGIN_DEN;
    let preceding: usize = heights[..selected].iter().sum();
    let furthest = heights.iter().sum::<usize>().saturating_sub(height);
    let target = preceding.saturating_sub(margin).min(furthest);

    let mut accumulated = 0;
    for (index, rows) in heights.iter().enumerate() {
        if accumulated + rows > target {
            return (index, target - accumulated);
        }
        accumulated += rows;
    }
    (selected, 0)
}

/// A thumb on a panel's right inner edge, when it has more rows than room.
///
/// Nothing is drawn when the content fits: a bar that is always full says nothing, and
/// its column would be taken from a panel that is already narrow.
///
/// **Call this after the panel's rows are rendered.** The band of a selected row runs
/// the full width of the panel, so a bar drawn first is painted over on exactly the row
/// the cursor is on: the thumb came out with a hole in it. The `bg` is reset rather than
/// inherited for the same reason — a thumb in the selection's green is a highlight of a
/// highlight.
pub fn bar(frame: &mut Frame, inner: Rect, total: usize, view: usize, scroll: usize) {
    if total <= view || view < 2 {
        return;
    }
    // the state is the number of scroll *positions*, not the number of rows: it travels
    // from `content_length - 1` plus a viewport, so handing it the row count makes the
    // thumb stop two thirds of the way down and stay there. One more than the furthest
    // scroll puts the top of the thumb at the top and its bottom at the bottom, which is
    // the only promise a thumb makes.
    let furthest = total - view;
    let mut state = ScrollbarState::new(furthest + 1)
        .viewport_content_length(view)
        .position(scroll.min(furthest));
    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(None)
            .thumb_symbol("▐")
            // no background: the band stops a column before this one (`band_end`), so
            // the thumb is drawn on the panel's own ground and never colours itself
            .style(Style::default().add_modifier(Modifier::DIM)),
        inner,
        &mut state,
    );
}

/// A row that has the cursor: the band runs to the end of the panel, not just under
/// the words. A highlight that stops mid-row reads as marked-up text; a row that is
/// filled reads as "the cursor is on this line".
pub fn banded(line: Line<'static>, width: usize) -> Line<'static> {
    let used = line.width();
    // the band flattens the row: every span takes the dark text, so a dim folder
    // name or a coloured tag does not end up unreadable on a light background
    let mut spans: Vec<ratatui::text::Span<'static>> = line
        .spans
        .into_iter()
        .map(|span| ratatui::text::Span::styled(span.content, super::consts::selection()))
        .collect();
    if used < width {
        spans.push(ratatui::text::Span::styled(
            " ".repeat(width - used),
            super::consts::selection(),
        ));
    }
    ratatui::text::Line::from(spans)
}

/// One todo as rows, in the shape the file has it: indentation, bullet and
/// checkbox as written, then the body wrapped between words — or, while it is
/// being edited, the text typed instead.
///
/// The markdown is not hidden. Someone reading a file of prose and todos reads the
/// bullets without being confused by them, and hiding one part of the line while
/// showing the rest is what makes a screen disagree with the document.
///
/// The highlight is the panel's: the whole todo carries it, continuation rows too,
/// so a wrapped todo reads as one item and not as the first row of one.
pub fn todo_rows(
    task: &Task,
    prompt: Option<&crate::state::prompt::Prompt>,
    selected: bool,
    width: usize,
) -> Vec<Line<'static>> {
    let body = prompt.map_or(task.body.as_str(), |prompt| prompt.text());
    line_rows(&task.lead(), body, selected, width)
}

/// A row whose band starts after a prefix: a heading's `##`, a history entry's dot and
/// lanes, a line number.
///
/// The prefix says *where* the row sits, not what it says, so it stays on the panel's
/// ground — a highlight over a line number reads as a highlighted line number. The band
/// still runs from the prefix to the panel's text edge, less the bar's column, because a
/// band that stops at the last character reads as marked-up text.
pub fn banded_after(prefix: Line<'static>, text: &str, width: usize) -> Line<'static> {
    let room = band_end(width).saturating_sub(prefix.width());
    let mut row = prefix;
    row.spans
        .extend(banded(Line::from(text.to_owned()), room).spans);
    row
}

/// The row a new todo is waiting in, while its field is open.
///
/// The line in the file is an empty checkbox — `- [ ]` and nothing else — which on its
/// own reads as a broken todo rather than an invitation. The dots are drawn, never
/// written: the file gets the title, not the hint.
pub fn waiting_rows(task: &Task, selected: bool, width: usize) -> Vec<Line<'static>> {
    line_rows(&task.lead(), "…", selected, width)
}

/// The same for a line that was read rather than parsed: a collection's item keeps
/// its `lead` as a string, because the file it came from is not the open document.
pub fn line_rows(
    lead: &str,
    body: &str,
    selected: bool,
    width: usize,
) -> Vec<Line<'static>> {
    // the continuation rows line up under the body, not under the bullet
    let pad = " ".repeat(UnicodeWidthStr::width(lead));
    wrap::lines(body, line_width(lead, width))
        .into_iter()
        .enumerate()
        .map(|(index, chunk)| {
            let prefix = if index == 0 { lead.to_owned() } else { pad.clone() };
            // in a panel of the file's own todos the line *is* the todo, bullet and
            // all, so the band covers it from the left edge
            let mut spans = vec![Span::styled(
                prefix,
                if selected {
                    super::consts::selection()
                } else {
                    Style::default()
                },
            )];
            spans.extend(styled(prompt::pieces(&chunk), selected));
            let line = Line::from(spans);
            if selected {
                pad_with_band(line, band_end(width))
            } else {
                line
            }
        })
        .collect()
}

/// How wide the body of this todo is on screen: the panel minus the bullet, the
/// checkbox and the indent in front of it.
pub fn body_width(task: &Task, width: usize) -> usize {
    line_width(&task.lead(), width)
}

/// The same, for a line that was read rather than parsed.
pub fn line_width(lead: &str, width: usize) -> usize {
    width.saturating_sub(UnicodeWidthStr::width(lead))
}

/// How many rows this body takes at this width — the same wrap the drawing uses,
/// so a scroll offset cannot drift from what is drawn.
pub fn row_count(task: &Task, body: &str, width: usize) -> usize {
    line_row_count(&task.lead(), body, width)
}

/// The same, for a line that was read rather than parsed.
pub fn line_row_count(lead: &str, body: &str, width: usize) -> usize {
    wrap::lines(body, line_width(lead, width)).len()
}

/// The spans of one piece of a row.
///
/// Under the cursor everything takes the band — except a tag or a person, which keep
/// being the name they were: their own colour behind them and white on top, so a row
/// still says what a todo is about while the cursor sits on it.
pub fn styled(pieces: Vec<(Span<'static>, Segment)>, selected: bool) -> Vec<Span<'static>> {
    pieces
        .into_iter()
        .map(|(mut span, role)| {
            if selected {
                span.style = match role {
                    Segment::Tag => super::consts::selection_name(super::prompt::TAG),
                    Segment::Assignee => super::consts::selection_name(super::prompt::ASSIGNEE),
                    Segment::Ticket => super::consts::selection_name(super::prompt::TICKET),
                    Segment::Title | Segment::Prio => super::consts::selection(),
                };
            }
            span
        })
        .collect()
}

/// Fill the rest of the row with the band, so a short todo still reads as a row.
pub fn pad_with_band(line: Line<'static>, width: usize) -> Line<'static> {
    let used = line.width();
    let mut line = line;
    if used < width {
        line.spans.push(Span::styled(
            " ".repeat(width - used),
            super::consts::selection(),
        ));
    }
    line
}

/// Put the terminal cursor inside a field: `lead` columns in from the left of
/// `inner`, then as far as the text typed in front of the cursor is wide. Clamped
/// so it never escapes the frame it belongs to.
pub fn cursor_in(frame: &mut Frame, inner: Rect, before_the_cursor: &str, lead: u16) {
    let column = as_u16(UnicodeWidthStr::width(before_the_cursor));
    let x = inner.x.saturating_add(lead).saturating_add(column);
    frame.set_cursor_position((x.min(inner.right().saturating_sub(1)), inner.y));
}

/// Where the terminal cursor belongs inside an edited, wrapped todo.
pub fn cursor_at(
    inner: Rect,
    first_row: usize,
    before_the_cursor: &str,
    width: usize,
    lead_width: usize,
) -> (u16, u16) {
    let chunks = wrap::lines(before_the_cursor, width.saturating_sub(lead_width));
    let rows_down = chunks.len().saturating_sub(1);
    let column = UnicodeWidthStr::width(chunks.last().map_or("", String::as_str));
    let x = inner.x.saturating_add(as_u16(lead_width + column));
    let y = inner
        .y
        .saturating_add(u16::try_from(first_row + rows_down).unwrap_or(u16::MAX));
    (
        x.min(inner.right().saturating_sub(1)),
        y.min(inner.bottom().saturating_sub(1)),
    )
}
