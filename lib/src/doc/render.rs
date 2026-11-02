//! Rendering: the lines back to text, and the numbers the sidebar shows.

use super::Doc;
use crate::model::Stats;

impl Doc {
    /// The file as text: lines joined with `\n`, one trailing newline.
    pub fn render(&self) -> String {
        if self.lines.is_empty() {
            return String::new();
        }
        let mut text = self.lines.join("\n");
        text.push('\n');
        text
    }

    /// How many tasks there are, and how many are done.
    pub fn stats(&self) -> Stats {
        let tasks = self.tasks();
        Stats {
            all: tasks.len(),
            done: tasks.iter().filter(|task| task.done).count(),
        }
    }

    /// One line by index.
    pub fn line(&self, index: usize) -> Option<&str> {
        self.lines.get(index).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::Doc;
    use crate::model::Stats;

    #[test]
    fn normalizes_crlf_and_strips_a_bom_on_render() {
        let doc = Doc::parse("\u{feff}- [ ] a\r\n- [x] b\r\n");
        assert_eq!(doc.tasks().len(), 2);
        assert_eq!(doc.render(), "- [ ] a\n- [x] b\n");
    }

    #[test]
    fn a_file_without_a_trailing_newline_gets_one() {
        assert_eq!(Doc::parse("- [ ] a").render(), "- [ ] a\n");
    }

    #[test]
    fn empty_lines_survive_a_round_trip() {
        let source = "# t\n\n- [ ] a\n\n";
        assert_eq!(Doc::parse(source).render(), source);
    }

    #[test]
    fn an_empty_file_renders_empty() {
        assert_eq!(Doc::parse("").render(), "");
    }

    #[test]
    fn stats_count_all_and_done() {
        let doc = Doc::parse("- [ ] a\n- [x] b\n- [x] c\n");
        assert_eq!(doc.stats(), Stats { all: 3, done: 2 });
    }

    #[test]
    fn line_reads_by_index() {
        let doc = Doc::parse("a\nb\n");
        assert_eq!(doc.line(1), Some("b"));
        assert_eq!(doc.line(2), None);
    }
}
