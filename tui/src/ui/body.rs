//! The file view: every line of the document, todos wrapped and coloured,
//! everything else dimmed and clipped to one line.

use std::collections::HashMap;

use aricma_todo_lib::model::Task;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::frame::{footer, panel};
use super::rows::{body_width, cursor_at, offset_for, row_count, todo_rows};
use crate::state::App;

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.pane == crate::state::action::Pane::View;
    let block =
        panel(&app.display(&app.current().path), focused).title_bottom(footer(app, focused));
    let inner = block.inner(area);
    let height = inner.height as usize;
    // the number column: the digits of the last line, and a space after it. The
    // whole document is numbered, prose included, so a todo seen here is the same
    // number it has in the editor and in the todos panel
    let gutter = gutter(app.doc.lines().len());
    let width = (inner.width as usize).saturating_sub(gutter);

    let tasks: HashMap<usize, Task> = app
        .tasks()
        .into_iter()
        .map(|task| (task.line, task))
        .collect();
    let lines = app.doc.lines();
    let selected = app.current_task().map(|task| task.line);
    // in a code file, the lines around a todo are context and the rest is the file:
    // one above, three below, and one `…` for what was folded away (RFC-018)
    let (keep, gap) = if app.comment() {
        folded(lines, &tasks)
    } else {
        (vec![true; lines.len()], vec![false; lines.len()])
    };
    let heights = heights(
        lines,
        &tasks,
        &keep,
        width,
        app.roomy.then_some(selected).flatten(),
    );
    let offset = offset_for(&heights, selected, height);

    let mut rows = Vec::new();
    let mut cursor = None;
    for index in offset.0..lines.len() {
        if rows.len() >= height {
            break;
        }
        let room = height - rows.len();
        let first = rows.len();
        let mut numbered = true;
        match tasks.get(&index) {
            Some(task) => {
                let editing = app.editing().filter(|_| Some(index) == selected);
                if let Some(prompt) = editing {
                    let before: String = prompt.text().chars().take(prompt.cursor()).collect();
                    let lead = width.saturating_sub(body_width(task, width));
                    cursor = Some(cursor_at(inner, rows.len(), &before, width, lead + gutter));
                }
                let here = selected == Some(index);
                // the row a new todo is waiting in says so, without the file knowing
                let waiting = app.add.is_some_and(|(line, _)| line == task.line);
                let rendered = if waiting {
                    super::rows::waiting_rows(task, here, width)
                } else {
                    todo_rows(task, editing, here, width)
                };
                let skip = if index == offset.0 { offset.1 } else { 0 };
                let room = room.saturating_sub(usize::from(app.roomy));
                rows.extend(rendered.into_iter().skip(skip).take(room));
                if app.roomy && rows.len() < height {
                    rows.push(Line::from(""));
                }
            }
            None if keep[index] => {
                rows.push(prose(lines.get(index).map_or("", String::as_str), width));
            }
            // the code that was folded away, said once and in one row
            None => {
                if gap[index] {
                    rows.push(gap_row(width));
                }
                numbered = false;
            }
        }
        if numbered {
            number(&mut rows[first..], index, gutter);
        }
    }
    frame.render_widget(Paragraph::new(rows).block(block), area);
    if let Some((x, y)) = cursor {
        frame.set_cursor_position((x, y));
    }
}

/// How many rows each document line needs: one, or more for a wrapped todo.
/// How many rows each document line needs: one, or more for a wrapped todo, and
/// two more for the todo that carries the cursor when the reader wants air around
/// it.
fn heights(
    lines: &[String],
    tasks: &HashMap<usize, Task>,
    keep: &[bool],
    width: usize,
    roomy: Option<usize>,
) -> Vec<usize> {
    lines
        .iter()
        .enumerate()
        .map(|(index, _)| {
            if !keep[index] {
                return 0;
            }
            let rows = tasks
                .get(&index)
                .map_or(1, |task| row_count(task, &task.body, width));
            if roomy == Some(index) {
                rows + 2
            } else {
                rows
            }
        })
        .collect()
}

/// How many lines of code are shown above and below a todo in a comment todo file.
///
/// Not the same number, because a comment todo is written *above* the code it is
/// about: the line above is the block the note sits in, and the lines below are the
/// change it asks for. Every developer writes the note first and the code after it,
/// which is the one habit this reader can lean on.
pub const ABOVE: usize = 1;
pub const BELOW: usize = 3;

/// Which lines of a comment todo file are worth a row, and where a run of code was
/// folded away.
///
/// Every todo keeps [`ABOVE`] lines above and [`BELOW`] below it, because the code
/// around a note is what the note is about. What is left between two of those windows
/// is one `…`, unless it is short enough to show — five lines or fewer, one line being
/// the `…` itself — because a fold that saves one line costs more than it saves.
fn folded(lines: &[String], tasks: &HashMap<usize, Task>) -> (Vec<bool>, Vec<bool>) {
    let mut keep = vec![false; lines.len()];
    for &at in tasks.keys() {
        let from = at.saturating_sub(ABOVE);
        let to = (at + BELOW).min(lines.len().saturating_sub(1));
        for line in &mut keep[from..=to] {
            *line = true;
        }
    }
    let mut gap = vec![false; lines.len()];
    let mut index = 0;
    while index < keep.len() {
        if keep[index] {
            index += 1;
            continue;
        }
        let start = index;
        while index < keep.len() && !keep[index] {
            index += 1;
        }
        if index - start <= ABOVE + BELOW + 1 {
            for line in &mut keep[start..index] {
                *line = true;
            }
        } else {
            gap[start] = true;
        }
    }
    (keep, gap)
}

/// The row that stands for the code between two windows of it.
fn gap_row(_width: usize) -> Line<'static> {
    Line::from("…").style(Style::default().add_modifier(Modifier::DIM))
}

/// How wide the line-number column is: the digits of the last line, and a space.
fn gutter(lines: usize) -> usize {
    lines.max(1).to_string().len() + 1
}

/// Put the line's number on its first row, and the column's width of air on the rows
/// it wraps onto, so the text of a wrapped line stays in one straight edge.
fn number(rows: &mut [Line<'static>], index: usize, gutter: usize) {
    for (at, row) in rows.iter_mut().enumerate() {
        let label = if at == 0 {
            format!("{:>width$} ", index + 1, width = gutter - 1)
        } else {
            " ".repeat(gutter)
        };
        row.spans.insert(
            0,
            Span::styled(label, Style::default().add_modifier(Modifier::DIM)),
        );
    }
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
            assert_eq!(
                offset_for(&heights, Some(selected), 24),
                (0, 0),
                "{selected}"
            );
        }
        // past it the viewport follows and the selection stays on row 7
        for selected in 8..40 {
            assert_eq!(
                offset_for(&heights, Some(selected), 24),
                (selected - 7, 0),
                "{selected}"
            );
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
            assert_eq!(preceding - rendered_before, expected, "selected {selected}");
        }
    }

    #[test]
    fn the_end_clamps_and_the_selection_walks_into_the_bottom() {
        let heights = vec![1; 60];
        // the last page is shown whole: no empty rows, and the selection ends up
        // on the last one
        assert_eq!(offset_for(&heights, Some(59), 24), (36, 0));
        assert_eq!(
            offset_for(&heights, Some(58), 24),
            (36, 0),
            "still the last page"
        );
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
