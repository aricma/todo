//! The collection view: every todo that carries one tag or one person, gathered
//! from the files they live in.
//!
//! It is a *view*, not a document: the heading at the top is rendered and never
//! written, and each todo is drawn under the file it came from, because an edit
//! here goes to that file and nowhere else. The file is a heading rather than a
//! prefix, once per file: forty todos from one file are one thing, not forty.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use super::frame::{footer, panel};
use super::rows::{cursor_at, offset_for};
use super::rows;
use crate::state::App;
use crate::text;

/// The collection heading and the blank row under it.
const HEADING_ROWS: usize = 2;

pub fn draw(frame: &mut Frame, app: &App, area: Rect, focused: bool) {
    let Some(collection) = app.viewing() else {
        return;
    };
    let title = format!("{} {}", collection, text::TODOS);
    let block = panel(&title, focused).title_bottom(footer(app, focused));
    let inner = block.inner(area);
    let width = inner.width as usize;
    let height = inner.height as usize;
    let gutter = gutter(app);

    let mut rows: Vec<Line<'static>> = Vec::new();
    // the heading is the collection's own name, so the panel says what it is even
    // after the frame's title has scrolled out of the eye
    rows.push(Line::from(Span::styled(
        text::collection_heading(kind(collection), &collection.name),
        Style::default().add_modifier(Modifier::DIM),
    )));
    rows.push(Line::from(""));

    // how tall each *item* is — its file's heading, the blank row over it and its
    // own lines — so the panel can scroll to the one under the cursor. The heading
    // and its blank row are rows like any other: counted here, or the scroll would
    // step straight over them
    let mut heights = vec![1; HEADING_ROWS];
    let mut before_the_cursor = 0;
    let mut last: Option<&std::path::Path> = None;
    for (index, item) in app.items().iter().enumerate() {
        let here = index == app.task_idx;
        let mut taken = 0;
        // a line break over every new file but the first: the heading's blank row
        // is already there, and two would be a gap with nothing to separate
        if last.is_some_and(|path| path != item.path) {
            rows.push(Line::from(""));
            taken += 1;
        }
        if last != Some(item.path.as_path()) {
            rows.push(Line::from(Span::styled(
                app.display(&item.path),
                Style::default().add_modifier(Modifier::DIM),
            )));
            taken += 1;
            last = Some(item.path.as_path());
        }
        if here {
            before_the_cursor = heights.iter().sum::<usize>();
            before_the_cursor += taken;
        }

        // the line is the line: indentation, bullet and checkbox as the file has
        // them, wrapped in what the number gutter leaves
        let editing = app.editing().filter(|_| here);
        let body = editing.map_or(item.body.as_str(), |prompt| prompt.text());
        for (at, mut row) in rows::line_rows(
            &item.lead,
            body,
            here,
            width.saturating_sub(gutter),
        )
        .into_iter()
        .enumerate()
        {
            // the number is a gutter, not part of the todo: it stays unbanded
            let number = if at == 0 {
                format!("{:>width$} ", item.line + 1, width = gutter - 1)
            } else {
                " ".repeat(gutter)
            };
            row.spans.insert(
                0,
                Span::styled(number, Style::default().add_modifier(Modifier::DIM)),
            );
            rows.push(row);
            taken += 1;
        }
        heights.push(taken);
    }

    let selected = app.task_idx + HEADING_ROWS;
    let (line, skip) = offset_for(&heights, Some(selected), height);
    let scroll: usize = heights.iter().take(line).sum::<usize>() + skip;
    frame.render_widget(
        Paragraph::new(rows)
            .scroll((u16::try_from(scroll).unwrap_or(u16::MAX), 0))
            .block(block),
        area,
    );

    if let Some((x, y)) = cursor(app, inner, width, gutter, before_the_cursor, scroll) {
        frame.set_cursor_position((x, y));
    }
}

/// How wide the line-number column is: the digits of the last line, and a space.
/// One column for the whole panel, so the files' lines sit at the same edge.
fn gutter(app: &App) -> usize {
    let last = app.items().last().map_or(1, |item| item.line + 1);
    last.to_string().len() + 1
}

/// Where the cursor goes while a collected todo is edited: `e` works here, so the
/// text being typed has to be visible here too.
fn cursor(
    app: &App,
    inner: Rect,
    width: usize,
    gutter: usize,
    first_row: usize,
    scroll: usize,
) -> Option<(u16, u16)> {
    let prompt = app.editing()?;
    let item = app.items().get(app.task_idx)?;
    let before: String = prompt.text().chars().take(prompt.cursor()).collect();
    let width = width.saturating_sub(gutter);
    // the body starts after the gutter and after the line's own indent and box
    let lead = gutter + UnicodeWidthStr::width(item.lead.as_str());
    Some(cursor_at(
        inner,
        first_row.saturating_sub(scroll),
        &before,
        width,
        lead,
    ))
}

/// What the heading calls the collection: `tag` or `person`.
fn kind(collection: &aricma_todo_lib::model::Collection) -> &'static str {
    match collection.kind {
        aricma_todo_lib::model::Kind::Tag => "tag",
        aricma_todo_lib::model::Kind::Assignee => "person",
    }
}
