//! The files section: one row per candidate file, where it lives, and its numbers.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget, Wrap};
use ratatui::Frame;

use super::consts::MIN_LIST_ROWS;
use super::consts::{message_prefix, message_style, WARNING};
use super::frame::panel;
use super::rows::offset_for;
use crate::helpers::{as_u16, split_path};
use crate::state::App;
use crate::text;

pub fn draw(frame: &mut Frame, app: &App, area: Rect, focused: bool) {
    let rows = rows(app, panel(text::FILES, focused).inner(area).width as usize);
    let block = panel(text::FILES, focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // a warning about the walk belongs to the list it explains, not to the status
    // bar, where it would push out the keys
    let (list, footer) = if app.warnings.is_empty() && app.panel_message().is_none() {
        (inner, None)
    } else {
        let notes = Paragraph::new(notes(app)).wrap(Wrap { trim: true });
        // never eat the list to say something about it
        let height = footer_height(&notes, inner).min(inner.height.saturating_sub(MIN_LIST_ROWS));
        let list = Rect {
            height: inner.height.saturating_sub(height),
            ..inner
        };
        let footer = Rect {
            y: inner.y + list.height,
            height,
            ..inner
        };
        (list, Some((notes, footer)))
    };

    // one row per line, so the list scrolls by the same rule as the file view: the
    // selection walks down its top third before the list moves at all. The blank
    // row belongs to the row above it, and collections are rows too — `file_idx`
    // would keep looking at the open file while the cursor walked past it.
    let heights = vec![if app.roomy { 2 } else { 1 }; app.rows.len()]; 
    let (first, _) = offset_for(&heights, Some(app.row_idx), list.height as usize);
    frame.render_widget(Paragraph::new(rows).scroll((as_u16(first), 0)), list);
    if let Some((notes, area)) = footer {
        frame.render_widget(notes, area);
    }
}

/// One row per file, in path order: the folders it lives in, grey, and its own
/// name in white.
///
/// The list is flat because the file is the only thing worth a row: a folder is
/// where a file is, not a thing you can do anything with.
/// One row per file, then one per collection: a tag or a person is a place todos
/// live, and the panel is where places are listed.
fn rows(app: &App, width: usize) -> Vec<Line<'static>> {
    let mut rows = Vec::with_capacity(app.rows.len());
    for (at, row) in app.rows.iter().enumerate() {
        let selected = at == app.row_idx;
        let line = match row {
            crate::state::row::Row::File(index) => file_row(app, *index, width),
            crate::state::row::Row::Collection(collection) => collection_row(collection, width),
        };
        rows.push(if selected {
            super::rows::banded(line, width)
        } else {
            line
        });
        if app.roomy && at + 1 < app.rows.len() {
            rows.push(Line::from(""));
        }
    }
    rows
}

/// `#home  2/5`: the name in the colour the todo panel gives it, and how much of it
/// is done — the same shape as a file row, because it is the same question.
fn collection_row(collection: &aricma_todo_lib::model::Collection, width: usize) -> Line<'static> {
    let colour = match collection.kind {
        aricma_todo_lib::model::Kind::Tag => super::prompt::TAG,
        aricma_todo_lib::model::Kind::Assignee => super::prompt::ASSIGNEE,
    };
    let line = Line::from(vec![
        Span::styled(
            format!("{}{}", collection.kind.sigil(), collection.name),
            Style::default().fg(colour),
        ),
        Span::styled(
            format!("  {}/{}", collection.done, collection.count),
            Style::default().add_modifier(Modifier::DIM),
        ),
    ]);
    let _ = width;
    line
}

fn file_row(app: &App, index: usize, _width: usize) -> Line<'static> {
    let file = &app.files[index];
    // a file over the line limit is listed, marked, and never opened
    let numbers = if file.too_long {
        "  too long".to_owned()
    } else {
        format!("  {}/{}", file.stats.done, file.stats.all)
    };
    let (folders, name) = split_path(&app.display(&file.path));
    let mut spans = Vec::new();
    if !folders.is_empty() {
        spans.push(Span::styled(
            folders,
            Style::default().add_modifier(Modifier::DIM),
        ));
    }
    spans.push(Span::raw(name));
    spans.push(Span::styled(
        numbers,
        Style::default().add_modifier(Modifier::DIM),
    ));
    // how many distinct tags and people the file mentions, in the colours the todo
    // panel gives them, so a file can be recognised by what it is about
    if file.stats.tags > 0 {
        spans.push(Span::styled(
            format!("  #{}", file.stats.tags),
            Style::default().fg(super::prompt::TAG),
        ));
    }
    if file.stats.assignees > 0 {
        spans.push(Span::styled(
            format!("  @{}", file.stats.assignees),
            Style::default().fg(super::prompt::ASSIGNEE),
        ));
    }
    // how many writes this session holds for the file: they are still undoable,
    // and the history panel is per file, so the list says which files have one
    let written = app.history_count(&file.path);
    if written > 0 {
        // orange, like the other warnings: it is the one thing on the row that is
        // worth a second look
        spans.push(Span::styled(
            format!("  h{written}"),
            Style::default().fg(WARNING),
        ));
    }
    let mut line = Line::from(spans);
    if file.too_long {
        line = line.style(Style::default().add_modifier(Modifier::DIM));
    }
    line
}

/// How many rows the footer takes once wrapped: measured by rendering it, because
/// the widget owns the wrapping rules and re-deriving them is how they drift.
fn footer_height(footer: &Paragraph, inner: Rect) -> u16 {
    let mut scratch = Buffer::empty(inner);
    footer.clone().render(scratch.area, &mut scratch);
    let width = scratch.area.width.max(1) as usize;
    scratch
        .content
        .iter()
        .rposition(|cell| cell.symbol() != " ")
        .map_or(0, |last| as_u16(last / width + 1))
}

/// What the walk stopped and what just failed, under the list they are about.
fn notes(app: &App) -> Vec<Line<'static>> {
    let mut rows = Vec::new();
    if let Some(message) = app.panel_message() {
        rows.push(Line::from(Span::styled(
            format!("{}{}", message_prefix(message.tone), message.text),
            message_style(message.tone),
        )));
    }
    rows.extend(warning_rows(app));
    rows
}

fn warning_rows(app: &App) -> Vec<Line<'static>> {
    app.warnings
        .iter()
        .map(|warning| {
            // the path a warning names is trimmed off here and kept in `--check`:
            // in a sidebar it is longer than the sentence it belongs to
            let summary = warning
                .split_once(" (first: ")
                .map_or(warning.as_str(), |(summary, _)| summary);
            // `[warn]` comes from the scan itself, so `--check` prints the same
            // line shape on stderr
            Line::from(Span::styled(
                summary.to_owned(),
                Style::default().fg(WARNING),
            ))
        })
        .collect()
}
