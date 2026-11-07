//! The history section: this file's writes, newest first, with the lines that
//! tie a write to the write it undoes.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::Frame;

use super::consts::message_style;
use super::consts::MIN_WRAP;
use super::wrap;
use crate::helpers::{as_u16, truncate};
use crate::state::message::Message;
use crate::state::App;
use crate::text;
use unicode_width::UnicodeWidthStr;

/// One lane of the graph: a dependency is a line running down the sidebar from
/// the write that undoes something to the write it undoes.
///
/// Box drawing, because a line that turns a corner reads as a connection: `╭` leaves
/// this row downwards, `│` keeps going, `╰` arrives and runs right into the row it
/// lands on.
const START: &str = "╭";
const PASS: &str = "│";
const ARRIVE: &str = "╰";
const RUN: &str = "─";
/// A lane running right through a row's horizontal run.
const CROSS: &str = "┼";

pub fn draw(frame: &mut Frame, app: &App, area: Rect, focused: bool) {
    let shown = app.file_history();
    let graph = graph(app, &shown);
    // a collection shows the history of the file the highlighted todo came from, so
    // the panel says which file that is
    let title = app.viewing().map_or_else(
        || text::titled(3, text::HISTORY),
        |_| text::history_of(&app.display(&app.current().path)),
    );
    let width = super::frame::panel(&title, focused).inner(area).width as usize;
    let mut rows = Vec::with_capacity(shown.len());
    // how many rows each entry takes, so a wrapped label can be scrolled to
    let mut heights = Vec::with_capacity(shown.len());
    for (index, global) in shown.iter().enumerate() {
        let Some(entry) = app.history.get(*global) else {
            continue;
        };
        // a filled dot is a write that is in the file, a hollow one is a write that was
        // undone: a checkbox here would read as a todo's state, and a write is not a todo
        let dot = if entry.applied { "●" } else { "○" };
        // the gutter is the same width on every row, so no label is pushed
        // sideways by a dependency it happens to be part of
        let style = if entry.applied {
            Style::default()
        } else {
            Style::default().add_modifier(Modifier::DIM)
        };
        let here = index == app.history_idx;
        let before = rows.len();
        if app.roomy {
            rows.push(Line::from(""));
        }
        // a long label wraps under itself, so a write is never cut in half: the
        // gutter and the checkbox stay on the first row and the rest lines up
        // under the text they continue
        let lead = format!("{}{dot} ", graph.row(index));
        // the rows a label wraps onto keep the lanes that run through this entry, so
        // a dependency line is a line and not a line with holes in it; the dot's
        // column stays blank, because a write has one state, on its first row
        let pad = format!("{}{}", graph.through(index), " ".repeat(2));
        // cells, not bytes: a lane is one cell and three bytes, and the ellipsis has to
        // land on the panel's edge rather than short of it
        let room = width.saturating_sub(UnicodeWidthStr::width(pad.as_str()));
        // one row by default, clipped with `…`; `h` wraps it. The trail is what the
        // panel is for, and one long write name pushes the rest of it off the panel
        let chunks = if app.history_one_line {
            vec![truncate(&entry.label, room.max(MIN_WRAP))]
        } else {
            wrap::lines(&entry.label, room.max(MIN_WRAP))
        };
        for (line, chunk) in chunks.into_iter().enumerate() {
            let prefix = if line == 0 { lead.clone() } else { pad.clone() };
            rows.push(if here {
                super::rows::banded_after(
                    Line::from(ratatui::text::Span::styled(prefix, style)),
                    &chunk,
                    width,
                )
            } else {
                Line::from(format!("{prefix}{chunk}")).style(style)
            });
        }
        // what this row cannot do, right under it: never in the status bar, and
        // gone as soon as the row stops being the one it is about
        if let Some(message) = app.message_for(*global) {
            rows.extend(message_rows(&graph, &message, area.width));
        }
        if app.roomy {
            rows.push(Line::from(""));
        }
        heights.push(rows.len() - before);
    }
    // the cursor can land on a wrapped entry near the bottom, so the panel follows
    // it: one row per line, so the offset is counted in rows
    let height = super::frame::panel(&title, focused).inner(area).height as usize;
    let (line, skip) = super::rows::offset_for(&heights, Some(app.history_idx), height);
    let scroll: usize = heights.iter().take(line).sum::<usize>() + skip;
    frame.render_widget(
        ratatui::widgets::Paragraph::new(rows)
            .scroll((as_u16(scroll), 0))
            .block(super::frame::panel(&title, focused)),
        area,
    );
    super::rows::bar(
        frame,
        super::frame::panel(&title, focused).inner(area),
        heights.iter().sum(),
        height,
        scroll,
    );
}

/// A hint in the row's own gutter, so it reads as a note on that row.
fn message_rows(graph: &Graph, message: &Message, width: u16) -> Vec<Line<'static>> {
    let lead = " ".repeat(graph.width + usize::from(graph.width > 0));
    let room = usize::from(width.saturating_sub(2)).saturating_sub(lead.len() + 2);
    wrap::lines(&message.text, room.max(MIN_WRAP))
        .into_iter()
        .enumerate()
        .map(|(index, text)| {
            let start = if index == 0 { "! " } else { "  " };
            Line::from(format!("{lead}{start}{text}")).style(message_style(message.tone))
        })
        .collect()
}

/// Where each dependency line runs.
struct Graph {
    /// One entry per row, one character per lane.
    rows: Vec<Vec<&'static str>>,
    /// The lane whose horizontal run owns the rest of this row, if any: the row a
    /// line leaves from, or the row it lands on.
    runs: Vec<Option<usize>>,
    width: usize,
}

impl Graph {
    /// The gutter of a row's continuation: only the lanes that run *through* this
    /// entry. A label that wraps is still the same write, so the pipes beside it
    /// carry on; a lane that arrived here has ended, and a gap is the truth.
    fn through(&self, row: usize) -> String {
        if self.width == 0 {
            return String::new();
        }
        let cells = self.rows.get(row).cloned().unwrap_or_default();
        let mut text = String::with_capacity(self.width + 1);
        for lane in 0..self.width {
            text.push_str(match cells.get(lane).copied().unwrap_or(" ") {
                START | PASS => PASS,
                _ => " ",
            });
        }
        text.push(' ');
        text
    }

    /// The gutter of one row, padded to the graph's width.
    ///
    /// A row where a line turns the corner gets a horizontal run from its lane to
    /// the label, so the line visibly reaches the write it is about.
    fn row(&self, row: usize) -> String {
        if self.width == 0 {
            return String::new();
        }
        let cells = self.rows.get(row).cloned().unwrap_or_default();
        let run = self.runs.get(row).copied().flatten();
        let mut text = String::with_capacity(self.width + 1);
        for lane in 0..self.width {
            let cell = cells.get(lane).copied().unwrap_or(" ");
            text.push_str(match (run, cell) {
                (Some(at), " ") if lane > at => RUN,
                (Some(at), PASS) if lane > at => CROSS,
                _ => cell,
            });
        }
        // the trailing column belongs to the label, so it carries the run into it
        text.push(if run.is_some() {
            RUN.chars().next().unwrap_or(' ')
        } else {
            ' '
        });
        text
    }
}

/// Lay the dependency lines out in lanes: each write hands its lane to the write
/// it undoes when that is the next row, so a chain of writes reads as one line.
fn graph(app: &App, shown: &[usize]) -> Graph {
    let target = |global: usize| {
        app.dependency(global)
            .and_then(|under| shown.iter().position(|index| *index == under))
    };
    let mut open: Vec<Option<usize>> = Vec::new();
    let mut rows: Vec<Vec<&'static str>> = Vec::with_capacity(shown.len());
    let mut runs: Vec<Option<usize>> = Vec::with_capacity(shown.len());
    let mut width = 0;
    for (row, global) in shown.iter().enumerate() {
        let mut cells: Vec<&'static str> = vec![" "; open.len()];
        let mut run = None;
        // a line that started above and ends at this row points at it
        let mut arrived = None;
        for (lane, ends) in open.iter_mut().enumerate() {
            if *ends == Some(row) {
                *ends = None;
                cells[lane] = ARRIVE;
                arrived = arrived.or(Some(lane));
                run = Some(lane);
            }
        }
        // this write's own line starts here, continuing the one it inherits
        if let Some(under) = target(*global) {
            let lane = if let Some(lane) = arrived {
                // the line that ended here carries on into the write it hangs on
                cells[lane] = PASS;
                lane
            } else {
                let lane = open.iter().position(Option::is_none).unwrap_or_else(|| {
                    open.push(None);
                    cells.push(" ");
                    open.len() - 1
                });
                cells[lane] = START;
                lane
            };
            run.get_or_insert(lane);
            open[lane] = Some(under);
        }
        for (lane, ends) in open.iter().enumerate() {
            if ends.is_some() && cells[lane] == " " {
                cells[lane] = PASS;
            }
        }
        width = width.max(open.len());
        rows.push(cells);
        runs.push(run);
    }
    Graph { rows, runs, width }
}
