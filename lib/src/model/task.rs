use crate::model::{Assignee, Prio, Tag};

/// One todo line, derived from a single line of the file.
///
/// `body` is the exact text after the checkbox, priority token included, so
/// every edit can rewrite just the part it owns and leave the rest untouched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Task {
    /// Zero-based index into `Doc::lines`.
    pub line: usize,
    /// Leading spaces; the indentation is preserved on every write.
    pub indent: usize,
    pub done: bool,
    pub prio: Option<Prio>,
    pub body: String,
    /// The body with the priority token and the tags/assignees set aside.
    pub title: String,
    pub tags: Vec<Tag>,
    pub assignees: Vec<Assignee>,
    /// The bullet that opened the line (`-`, `*`, `+`, `1.`), kept verbatim.
    pub(crate) marker: String,
}

impl Task {
    /// Everything in front of the body: indentation, bullet, checkbox, and the
    /// space after it. The part of the line this tool does not own.
    ///
    /// A screen shows it, because a todo should read like the line it is: the
    /// bullet the author wrote is not noise to be hidden from someone who is
    /// already reading the rest of the document.
    pub fn lead(&self) -> String {
        let checkbox = if self.done { 'x' } else { ' ' };
        format!("{}{} [{checkbox}] ", " ".repeat(self.indent), self.marker)
    }

    /// The line as written back: indent, bullet, checkbox, body.
    ///
    /// The body goes out exactly as it came in, so a toggle cannot disturb the
    /// parts of the line this tool does not own.
    pub fn render(&self) -> String {
        format!("{}{}", self.lead(), self.body)
    }
}

#[cfg(test)]
mod tests {
    use crate::doc::Doc;

    #[test]
    fn render_round_trips_a_line_with_every_kind_of_body() {
        for source in [
            "- [ ] (A)  two  spaces #x  @y\n",
            "  * [x] @dave fix you #worst-bug-of-the-century\n",
            "1. [ ] plain\n",
            "+ [X] (d) a #café\n",
        ] {
            let doc = Doc::parse(source);
            let task = &doc.tasks()[0];
            let expected = source.trim_end().replace("[X]", "[x]");
            assert_eq!(task.render(), expected, "{source:?}");
        }
    }

    #[test]
    fn the_lead_is_the_line_minus_its_body() {
        let doc = Doc::parse("  * [x] (A) buy milk\n1. [ ] plain\n");
        let tasks = doc.tasks();
        assert_eq!(tasks[0].lead(), "  * [x] ");
        assert_eq!(tasks[1].lead(), "1. [ ] ");
        for task in &tasks {
            assert_eq!(format!("{}{}", task.lead(), task.body), task.render());
        }
    }
}
