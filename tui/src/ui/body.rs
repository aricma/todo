//! The file view: every line of the document, todos wrapped and coloured,
//! everything else dimmed and clipped to one line.

use std::collections::HashMap;

use aricma_todo_lib::model::Task;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::super::App;
use super::{cursor_at, editing_rows, panel, task_rows, CHECKBOX_WIDTH};

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.pane == crate::action::Pane::View;
    let block = panel(&app.display(&app.current().path), focused);
    let inner = block.inner(area);
    let height = inner.height as usize;
    let width = inner.width as usize;

    let tasks: HashMap<usize, Task> = app
        .tasks()
        .into_iter()
        .map(|task| (task.line, task))
        .collect();
    let lines = app.doc.lines();
    let heights = heights(lines, &tasks, width);
    let selected = app.current_task().map(|task| task.line);
    let offset = offset_for(&heights, selected, height);

    let mut rows = Vec::new();
    let mut cursor = None;
    for index in offset.0..lines.len() {
        if rows.len() >= height {
            break;
        }
        let room = height - rows.len();
        match tasks.get(&index) {
            Some(task) => {
                let editing = app.editing().filter(|_| Some(index) == selected);
                let rendered = match editing {
                    Some(prompt) => {
                        let before: String = prompt.text().chars().take(prompt.cursor()).collect();
                        let first = rows.len();
                        cursor = Some(cursor_at(inner, first, &before, width));
                        editing_rows(task, prompt, width)
                    }
                    None => task_rows(task, width),
                };
                let skip = if index == offset.0 { offset.1 } else { 0 };
                for row in rendered.into_iter().skip(skip).take(room) {
                    // the highlight covers the whole todo, continuation rows too
                    let row = if selected == Some(index) {
                        row.style(Style::default().add_modifier(Modifier::REVERSED))
                    } else {
                        row
                    };
                    rows.push(row);
                }
            }
            None => rows.push(prose(lines.get(index).map_or("", String::as_str), width)),
        }
    }
    frame.render_widget(Paragraph::new(rows).block(block), area);
    if let Some((x, y)) = cursor {
        frame.set_cursor_position((x, y));
    }
}

/// How many rows each document line needs: one, or more for a wrapped todo.
fn heights(lines: &[String], tasks: &HashMap<usize, Task>, width: usize) -> Vec<usize> {
    let body_width = width.saturating_sub(CHECKBOX_WIDTH);
    lines
        .iter()
        .enumerate()
        .map(|(index, _)| {
            tasks
                .get(&index)
                .map_or(1, |task| super::wrap::lines(&task.body, body_width).len())
        })
        .collect()
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
fn offset_for(heights: &[usize], selected: Option<usize>, height: usize) -> (usize, usize) {
    let (Some(selected), true) = (selected, height > 0) else {
        return (0, 0);
    };
    let selected = selected.min(heights.len().saturating_sub(1));
    let margin = height * 3 / 10;
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

/// Prose: dim, one line, clipped with an ellipsis.
fn prose(raw: &str, width: usize) -> Line<'static> {
    Line::from(truncate(raw.trim_end(), width)).style(Style::default().add_modifier(Modifier::DIM))
}

/// Clip to a display width, marking the cut with `…`.
fn truncate(text: &str, width: usize) -> String {
    if UnicodeWidthStr::width(text) <= width {
        return text.to_owned();
    }
    let mut out = String::new();
    let mut used = 0;
    for character in text.chars() {
        let character_width = UnicodeWidthChar::width(character).unwrap_or(0);
        if used + character_width + 1 > width {
            break;
        }
        out.push(character);
        used += character_width;
    }
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use unicode_width::UnicodeWidthStr;

    use super::{offset_for, truncate};

    #[test]
    fn text_that_fits_is_untouched() {
        assert_eq!(truncate("short", 10), "short");
    }

    #[test]
    fn long_text_is_clipped_with_an_ellipsis() {
        let clipped = truncate(&"a".repeat(400), 10);
        assert_eq!(clipped.chars().count(), 10);
        assert!(clipped.ends_with('…'));
    }

    #[test]
    fn wide_characters_are_measured_by_display_width() {
        let clipped = truncate("日本語のテキスト", 8);
        assert!(UnicodeWidthStr::width(clipped.as_str()) <= 8, "{clipped}");
    }

    #[test]
    fn the_top_third_is_a_margin_that_does_not_scroll() {
        let heights = vec![1; 60];
        // 24 rows, a margin of 7: the selection walks down the panel until row 7
        for selected in 0..=7 {
            assert_eq!(offset_for(&heights, Some(selected), 24), (0, 0), "{selected}");
        }
        // past it the viewport follows and the selection stays on row 7
        for selected in 8..40 {
            assert_eq!(offset_for(&heights, Some(selected), 24), (selected - 7, 0), "{selected}");
        }
        assert_eq!(offset_for(&heights, None, 24), (0, 0));
        assert_eq!(offset_for(&heights, Some(5), 0), (0, 0));
    }

    #[test]
    fn a_wrapped_neighbour_does_not_move_the_selection() {
        // row heights: two one-row lines, a three-row todo, then the rest
        let heights = [1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1];
        let height = 10;
        let margin = height * 3 / 10;
        // the checkmark row is the margin for everything below it, no matter how
        // tall the line above was — until the document clamps at its last page
        let total: usize = heights.iter().sum();
        let furthest = total - height;
        for selected in 0..heights.len() {
            let (line, skip) = offset_for(&heights, Some(selected), height);
            let rendered_before: usize = heights[..line].iter().sum::<usize>() + skip;
            let preceding: usize = heights[..selected].iter().sum();
            let expected = preceding - preceding.saturating_sub(margin).min(furthest);
            assert_eq!(
                preceding - rendered_before,
                expected,
                "selected {selected}"
            );
        }
    }

    #[test]
    fn the_end_clamps_and_the_selection_walks_into_the_bottom() {
        let heights = vec![1; 60];
        // the last page is shown whole: no empty rows, and the selection ends up
        // on the last one
        assert_eq!(offset_for(&heights, Some(59), 24), (36, 0));
        assert_eq!(offset_for(&heights, Some(58), 24), (36, 0), "still the last page");
        // far from the end the margin still wins
        assert_eq!(offset_for(&heights, Some(40), 24), (33, 0));
        // near it the clamp takes over and the selection walks down the panel
        assert_eq!(offset_for(&heights, Some(50), 24), (36, 0));
    }

    #[test]
    fn a_wrapped_todo_taller_than_the_panel_starts_at_itself() {
        // a 3-row todo in a 2-row panel: no margin to spend, so it starts on it
        assert_eq!(offset_for(&[1, 1, 3, 1], Some(2), 2), (2, 0));
        // and when it does fit with its neighbours, the top is the answer
        assert_eq!(offset_for(&[1, 3, 1, 1], Some(1), 4), (0, 0));
    }
}
