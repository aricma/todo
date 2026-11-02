//! A body as styled spans, and the colours that mean what.

use aricma_todo_lib::doc::spans;
use aricma_todo_lib::model::Segment;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Priority, tag and assignee colours. Deliberately few: a title is uncoloured.
pub const PRIO: Color = Color::Yellow;
pub const TAG: Color = Color::Cyan;
pub const ASSIGNEE: Color = Color::Magenta;

/// The style for one recognised piece.
pub fn style(segment: Segment) -> Style {
    match segment {
        Segment::Prio => Style::default().fg(PRIO).add_modifier(Modifier::BOLD),
        Segment::Tag => Style::default().fg(TAG),
        Segment::Assignee => Style::default().fg(ASSIGNEE),
        Segment::Title => Style::default(),
    }
}

/// A body with its priority, tags and assignees coloured.
///
/// The gaps between recognised pieces are plain, so rendering this line always
/// shows exactly the text that was typed.
pub fn line(body: &str) -> Line<'static> {
    let mut pieces = Vec::new();
    let mut cursor = 0;
    for (segment, range) in spans::segments(body) {
        if range.start > cursor {
            pieces.push(Span::raw(body[cursor..range.start].to_owned()));
        }
        pieces.push(Span::styled(body[range.clone()].to_owned(), style(segment)));
        cursor = range.end;
    }
    if cursor < body.len() {
        pieces.push(Span::raw(body[cursor..].to_owned()));
    }
    Line::from(pieces)
}

#[cfg(test)]
mod tests {
    use aricma_todo_lib::model::Segment;

    use super::{line, style, ASSIGNEE, PRIO, TAG};

    #[test]
    fn recognised_pieces_get_their_colour_and_title_does_not() {
        assert_eq!(style(Segment::Prio).fg, Some(PRIO));
        assert_eq!(style(Segment::Tag).fg, Some(TAG));
        assert_eq!(style(Segment::Assignee).fg, Some(ASSIGNEE));
        assert_eq!(style(Segment::Title).fg, None);
    }

    #[test]
    fn the_line_still_reads_as_the_body() {
        let body = "(B) @dave fix you #bug";
        let rendered: String = line(body)
            .spans
            .iter()
            .map(|span| span.content.clone())
            .collect();
        assert_eq!(rendered, body);
    }

    #[test]
    fn unknown_tokens_stay_plain() {
        let body = "ship #café";
        let line = line(body);
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].style.fg, None);
    }

    #[test]
    fn an_empty_body_is_one_empty_line() {
        assert_eq!(line("").spans.len(), 0);
    }
}
