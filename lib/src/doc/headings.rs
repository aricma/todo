//! The document's outline: its headings, in file order.
//!
//! `#`…`######` with at least one space after, up to three leading spaces, never
//! inside a fence. Nothing else counts.

use super::{parse, Doc};
use crate::model::Heading;

impl Doc {
    /// Every heading in the file, in order.
    pub fn headings(&self) -> Vec<Heading> {
        let fenced = parse::fenced(&self.lines);
        self.lines
            .iter()
            .enumerate()
            .filter(|(index, _)| !fenced[*index])
            .filter_map(|(index, line)| heading(line, index))
            .collect()
    }
}

pub(super) fn heading(line: &str, index: usize) -> Option<Heading> {
    let trimmed = line.trim_start_matches(' ');
    if line.len() - trimmed.len() > 3 {
        return None;
    }
    let level = trimmed
        .chars()
        .take_while(|character| *character == '#')
        .count();
    if level == 0 || level > 6 {
        return None;
    }
    let rest = trimmed[level..].strip_prefix(' ')?;
    let text = rest.trim().trim_end_matches('#').trim_end();
    if text.is_empty() {
        return None;
    }
    Some(Heading {
        line: index,
        level: u8::try_from(level).unwrap_or(6),
        text: text.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::Doc;

    fn outline(source: &str) -> Vec<(usize, u8, String)> {
        Doc::parse(source)
            .headings()
            .into_iter()
            .map(|heading| (heading.line, heading.level, heading.text))
            .collect()
    }

    #[test]
    fn finds_headings_with_their_level_and_line() {
        let source = "# TODOs\n\n## inbox\n\n- [ ] a\n\n### later\n";
        assert_eq!(
            outline(source),
            [
                (0, 1, "TODOs".to_owned()),
                (2, 2, "inbox".to_owned()),
                (6, 3, "later".to_owned()),
            ]
        );
    }

    #[test]
    fn ignores_hashes_that_are_not_headings() {
        for line in [
            "#nospace\n",
            "####### too deep\n",
            "#\n",
            "    # four spaces\n",
        ] {
            assert!(outline(line).is_empty(), "{line:?}");
        }
    }

    #[test]
    fn three_leading_spaces_are_still_a_heading() {
        assert_eq!(outline("   # indented\n"), [(0, 1, "indented".to_owned())]);
    }

    #[test]
    fn ignores_headings_inside_a_fence() {
        assert!(outline("```\n# not a heading\n```\n").is_empty());
    }

    #[test]
    fn drops_the_closing_hashes() {
        assert_eq!(outline("## done ##\n"), [(0, 2, "done".to_owned())]);
    }

    #[test]
    fn a_document_without_headings_has_an_empty_outline() {
        assert!(outline("- [ ] a\n- [ ] b\n").is_empty());
    }
}
