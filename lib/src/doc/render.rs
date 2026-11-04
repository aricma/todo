//! Rendering: the lines back to text, and the numbers the sidebar shows.

use super::Doc;
use crate::model::{Collection, Kind, Stats};

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

    /// The collections this file's todos belong to, by name and with how many todos
    /// carry each. Tags first, then people, each by name.
    pub fn collections(&self) -> Vec<Collection> {
        use std::collections::BTreeMap;

        let tasks = self.tasks();
        // all, then done
        let mut found: BTreeMap<(Kind, String), (usize, usize)> = BTreeMap::new();
        for task in &tasks {
            // one todo counts once, however often it writes the same name
            let names = task
                .tags
                .iter()
                .map(|tag| (Kind::Tag, tag.as_str()))
                .chain(
                    task.assignees
                        .iter()
                        .map(|who| (Kind::Assignee, who.as_str())),
                )
                .collect::<std::collections::BTreeSet<_>>();
            for (kind, name) in names {
                let counts = found.entry((kind, name.to_owned())).or_default();
                counts.0 += 1;
                if task.done {
                    counts.1 += 1;
                }
            }
        }
        found
            .into_iter()
            .map(|((kind, name), (count, done))| Collection {
                kind,
                name,
                count,
                done,
            })
            .collect()
    }

    /// How many tasks there are, how many are done, and how many distinct tags and
    /// people the file mentions.
    ///
    /// Distinct, not counted per occurrence: "this file mentions three tags" is what
    /// a person reading a file list wants to know, and a tag written on nine lines is
    /// still one tag.
    pub fn stats(&self) -> Stats {
        let tasks = self.tasks();
        let mut tags = std::collections::BTreeSet::new();
        let mut assignees = std::collections::BTreeSet::new();
        for task in &tasks {
            tags.extend(task.tags.iter().map(crate::model::Tag::as_str));
            assignees.extend(task.assignees.iter().map(crate::model::Assignee::as_str));
        }
        Stats {
            all: tasks.len(),
            done: tasks.iter().filter(|task| task.done).count(),
            tags: tags.len(),
            assignees: assignees.len(),
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
    fn stats_count_all_done_and_the_distinct_tags_and_people() {
        let doc = Doc::parse("- [ ] a #home #home @sam\n- [x] b #work @sam\n- [x] c\n");
        assert_eq!(
            doc.stats(),
            Stats {
                all: 3,
                done: 2,
                // a tag written twice is still one tag, and so is a person
                tags: 2,
                assignees: 1
            }
        );
    }

    #[test]
    fn line_reads_by_index() {
        let doc = Doc::parse("a\nb\n");
        assert_eq!(doc.line(1), Some("b"));
        assert_eq!(doc.line(2), None);
    }
}
