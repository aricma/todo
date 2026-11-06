//! Parsing: a file becomes exact lines, and a line becomes a `Task`.
//!
//! `scan` is the single place that decides what counts as a task, so `tasks`,
//! `findings` and `format` can never disagree about it.

use super::spans;
use super::{Doc, Source};
use crate::model::Task;

impl Doc {
    /// Split a file into lines. CRLF loses its `\r`, a BOM is dropped, and the
    /// trailing newline is not a line of its own.
    pub fn parse(source: &str) -> Self {
        Self {
            lines: lines(source),
            source: Source::Document,
        }
    }

    /// The same, for a file whose todos are `TODO:` comments: any file in the tree
    /// that is not a todo document. Its lines are kept exactly as they are, so the
    /// code around a todo is neither moved nor rewritten (RFC-016).
    pub fn parse_comment(source: &str) -> Self {
        Self {
            lines: lines(source),
            source: Source::Comment,
        }
    }

    /// Every task in the file, in file order.
    pub fn tasks(&self) -> Vec<Task> {
        match self.source {
            Source::Document => scan(&self.lines),
            Source::Comment => scan_comment(&self.lines),
        }
    }
}

fn lines(source: &str) -> Vec<String> {
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    let mut lines: Vec<String> = source
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line).to_owned())
        .collect();
    if source.ends_with('\n') || source.is_empty() {
        lines.pop();
    }
    lines
}

/// Walk the lines of a code file and collect its `TODO:` comments.
///
/// No fences and no headings: a code file is not markdown, and `# TODO:` in a shell
/// script is a comment, not an outline. One todo per line, the first `TODO:` on it,
/// and the code around it is the todo's lead (RFC-016).
fn scan_comment(lines: &[String]) -> Vec<Task> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| comment_line(line, index))
        .collect()
}

/// One line as a comment todo, or `None`.
///
/// The rule is the literal `TODO:` in capitals, with at least one character after
/// it that is not a space: a bare `TODO:` promises nothing and is not a todo, and a
/// lowercase `todo:` is a sentence talking about one — "a file that no longer holds a
/// todo: the bar says only what works".
fn comment_line(line: &str, index: usize) -> Option<Task> {
    // the first `TODO:` that a comment marker stands in front of, because the one in
    // `let note = "TODO:"` is a string
    let (at, _) = line
        .match_indices("TODO:")
        .find(|(at, _)| marked(&line[..*at]))?;
    let colon = at + "TODO:".len();
    // the body starts after the colon and the one space that separates it, if there
    // is one: the rest is the line's, whitespace and all
    let start = if line[colon..].starts_with(' ') {
        colon + 1
    } else {
        colon
    };
    let body = line[start..].trim();
    if body.is_empty() {
        return None;
    }
    let parts = spans::parts(body);
    Some(Task {
        line: index,
        indent: line.len() - line.trim_start_matches(' ').len(),
        done: false,
        prio: parts.prio,
        title: parts.title,
        body: body.to_owned(),
        tags: parts.tags,
        assignees: parts.assignees,
        marker: String::new(),
        prefix: Some(line[..start].to_owned()),
    })
}

/// Whether a comment marker ends the text in front of a `TODO:` — or the line
/// starts there, which is how a plain `.txt` list is written.
///
/// Without this, prose about the rule is a todo of its own: `/// todos are
/// `TODO:` comments` in this very file listed `hints.rs` for a todo it does not
/// have. The markers are the ones a language starts a comment with — `#`, `/`,
/// `-`, `;`, `%`, `*` — which is all of `//`, `///`, `/*`, `#!`, `--`, `<!--`
/// and a `*` continuing a block.
///
/// A quote or a backtick in front of the marker means the marker is quoted: it is
/// inside a string literal, or in prose about code. `Doc::parse_comment("// TODO:
/// x")` is a test fixture, not a todo, and `` — a `<!-- TODO: -->` `` in a
/// sentence is a sentence (RFC-016).
fn marked(lead: &str) -> bool {
    let lead = lead.trim_end();
    let quoted = lead.chars().any(|c| matches!(c, '"' | '\'' | '`'));
    !quoted
        && (lead.is_empty()
            || matches!(lead.chars().next_back(), Some('#' | '/' | '-' | ';' | '%' | '*')))
}

/// Walk the lines, skipping fenced code blocks, and collect the tasks.
fn scan(lines: &[String]) -> Vec<Task> {
    let fenced = fenced(lines);
    lines
        .iter()
        .enumerate()
        .filter(|(index, _)| !fenced[*index])
        .filter_map(|(index, line)| parse_line(line, index))
        .collect()
}

/// Which lines sit inside a fenced code block. The fence lines themselves do
/// not: they are structure, not content.
pub(crate) fn fenced(lines: &[String]) -> Vec<bool> {
    let mut flags = Vec::with_capacity(lines.len());
    let mut fence: Option<&'static str> = None;
    for line in lines {
        flags.push(fence.is_some());
        if let Some(marker) = fence_of(line) {
            fence = match fence {
                Some(open) if open == marker => None,
                Some(open) => Some(open),
                None => Some(marker),
            };
        }
    }
    flags
}

/// The fence marker a line opens or closes, when it is a fence line.
fn fence_of(line: &str) -> Option<&'static str> {
    let trimmed = line.trim_start_matches(' ');
    if line.len() - trimmed.len() > 3 {
        return None;
    }
    if trimmed.starts_with("```") {
        Some("```")
    } else if trimmed.starts_with("~~~") {
        Some("~~~")
    } else {
        None
    }
}

/// One line as a task, or `None` when it is prose.
pub(crate) fn parse_line(line: &str, index: usize) -> Option<Task> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    let rest = &line[indent..];
    let (marker, after) = bullet(rest)?;
    let after = after.strip_prefix(' ')?;
    let after = after.strip_prefix('[')?;
    let mut chars = after.chars();
    let checkbox = chars.next()?;
    let done = match checkbox {
        ' ' => false,
        'x' | 'X' => true,
        _ => return None,
    };
    let after = chars.as_str().strip_prefix(']')?;
    let body = match after.strip_prefix(' ') {
        Some(rest) => rest.trim(),
        // `- [ ]` with nothing after it is the row a new todo waits in before its
        // title is typed; `[ ]text` is still not a task, because the space after
        // the checkbox is what separates it from the title (RFC-013)
        None if after.trim().is_empty() => "",
        None => return None,
    };
    let parts = spans::parts(body);
    Some(Task {
        line: index,
        indent,
        done,
        prio: parts.prio,
        title: parts.title,
        body: body.to_owned(),
        tags: parts.tags,
        assignees: parts.assignees,
        marker,
        prefix: None,
    })
}

/// The bullet that opens a task line, and the text after it.
pub(crate) fn bullet(rest: &str) -> Option<(String, &str)> {
    for marker in ["-", "*", "+"] {
        if let Some(after) = rest.strip_prefix(marker) {
            return Some((marker.to_owned(), after));
        }
    }
    let digits = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    if digits == 0 {
        return None;
    }
    let after_digits = &rest[digits..];
    let marker = after_digits
        .chars()
        .next()
        .filter(|c| *c == '.' || *c == ')')?;
    Some((format!("{}{marker}", &rest[..digits]), &after_digits[1..]))
}

#[cfg(test)]
mod tests {
    use super::Doc;
    use crate::model::Prio;

    #[test]
    fn a_comment_todo_file_finds_its_todo_comments_and_nothing_else() {
        let doc = Doc::parse_comment(
            "fn main() {\n    // TODO: ship it\n    // FIXME: not this\n    // todo: nor this\n}\n",
        );
        let tasks = doc.tasks();
        assert_eq!(tasks.len(), 1, "{tasks:#?}");
        assert_eq!(tasks[0].line, 1);
        assert_eq!(tasks[0].title, "ship it");
        assert_eq!(tasks[0].lead(), "    // TODO: ");
    }

    #[test]
    fn a_comment_todo_needs_something_after_the_colon() {
        assert!(Doc::parse_comment("// TODO:\n// TODO:   \n").tasks().is_empty());
        assert_eq!(Doc::parse_comment("// TODO:x\n").tasks()[0].body, "x");
    }

    #[test]
    fn a_checkbox_in_a_code_file_is_code_and_a_comment_in_markdown_is_prose() {
        assert!(Doc::parse_comment("- [ ] not a todo here\n").tasks().is_empty());
        assert!(Doc::parse("// TODO: not a todo here\n").tasks().is_empty());
    }

    #[test]
    fn a_todo_needs_a_comment_marker_in_front_of_it() {
        // prose about the rule is not a todo, which is the bug this fixed:
        // hints.rs was listed because a doc comment said `TODO:` in a sentence
        assert!(Doc::parse_comment("/// the todos are `TODO:` comments\n")
            .tasks()
            .is_empty());
        assert!(Doc::parse_comment("let note = \"TODO: buy milk\";\n")
            .tasks()
            .is_empty());
        // lowercase is a sentence about a todo, not a todo: this is the noise that
        // made a test comment in hints.rs a todo of its own
        assert!(Doc::parse_comment("// a file with no todo: the bar says less\n")
            .tasks()
            .is_empty());
        // a marker in a string, or in a sentence about code, is quoted, not a comment
        assert!(Doc::parse_comment("Doc::parse_comment(\"// TODO: x\")\n")
            .tasks()
            .is_empty());
        assert!(Doc::parse_comment("/// a `<!-- TODO: -->` in a sentence\n")
            .tasks()
            .is_empty());
        // the markers every language starts a comment with, and a `*` inside a block
        for line in [
            "# TODO: shell\n",
            "/// TODO: rust\n",
            "/* TODO: c */\n",
            " * TODO: block\n",
            "-- TODO: sql\n",
            "; TODO: lisp\n",
            "% TODO: tex\n",
            "<!-- TODO: html -->\n",
        ] {
            assert_eq!(Doc::parse_comment(line).tasks().len(), 1, "{line:?}");
        }
        // a bare line is a list in a notes file, indented or not
        assert_eq!(Doc::parse_comment("TODO: plain\n  TODO: indented\n").tasks().len(), 2);
    }

    /// The table in RFC-019, line for line. If a condition changes, this fails
    /// before the RFC lies.
    #[test]
    fn the_rule_is_narrow_and_its_misses_are_known() {
        let listed = [
            "    // TODO: pay rent",
            "# TODO: pay rent",
            "-- TODO: pay rent",
            " * TODO: pay rent",
            "/* TODO: pay rent */",
            "<!-- TODO: pay rent -->",
            "TODO: pay rent",
            "  TODO: pay rent",
            "let a = 1; // TODO: pay rent",
            "let x = 1; //TODO: pay rent",
            // a quote after the colon is the todo's own text, not a quote in front
            "// TODO: a \"quoted\" word",
        ];
        let missed = [
            // the quote rule: the marker is quoted
            "let a = \"x\"; // TODO: pay rent",
            "\"// TODO: pay rent\"",
            "/// a `TODO:` note",
            // the capitals rule
            "// todo: pay rent",
            "// Todo: pay rent",
            // the colon rule
            "// TODO pay rent",
            "// TODO(bob): pay rent",
            "// FIXME: pay rent",
            "// XXX: pay rent",
            // the body rule
            "// TODO:",
            "// TODO:   ",
        ];
        for line in listed {
            assert_eq!(
                Doc::parse_comment(&format!("{line}\n")).tasks().len(),
                1,
                "should be listed: {line:?}"
            );
        }
        for line in missed {
            assert_eq!(
                Doc::parse_comment(&format!("{line}\n")).tasks().len(),
                0,
                "should be missed: {line:?}"
            );
        }
        // the false positive that stays: prose writing the marker unquoted, which no
        // line rule can tell from an HTML comment
        assert_eq!(
            Doc::parse_comment("/// we use <!-- TODO: --> in prose\n").tasks().len(),
            1
        );
    }

    #[test]
    fn a_string_before_a_trailing_comment_hides_it() {
        // the cost of the quote rule: a note after code that quotes something is
        // missed, because the tool cannot tell which side of the quote it is on
        assert!(Doc::parse_comment("let a = \"x\"; // TODO: this\n").tasks().is_empty());
    }

    #[test]
    fn a_comment_todo_reads_the_priority_the_tags_and_the_people() {
        let doc = Doc::parse_comment("/* TODO: (A) fix #infra @dave */\n");
        let task = &doc.tasks()[0];
        assert_eq!(task.prio, Some(Prio::A));
        // the terminator is the code's, not the tool's: it stays in the text
        assert_eq!(task.title, "fix */");
        assert_eq!(task.tags[0].as_str(), "infra");
        assert_eq!(task.assignees[0].as_str(), "dave");
        assert_eq!(task.body, "(A) fix #infra @dave */");
    }

    #[test]
    fn parses_checked_unchecked_and_indented_tasks() {
        let doc = Doc::parse("- [ ] buy milk\n  - [x] pay rent\n");
        let tasks = doc.tasks();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].title, "buy milk");
        assert!(!tasks[0].done);
        assert!(tasks[1].done);
        assert_eq!(tasks[1].indent, 2);
        assert_eq!(tasks[1].line, 1);
    }

    #[test]
    fn ignores_prose_and_keeps_it_verbatim() {
        let source = "# TODOs\n\nsome prose - [not a task\n  - [ ] nested\n";
        let doc = Doc::parse(source);
        let tasks = doc.tasks();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].line, 3);
        assert_eq!(tasks[0].indent, 2);
        assert_eq!(doc.render(), source);
    }

    #[test]
    fn a_task_inside_a_fence_is_prose() {
        let doc = Doc::parse("```md\n- [ ] example\n```\n- [ ] real\n~~~\n- [ ] tilde\n~~~\n");
        let tasks = doc.tasks();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "real");
        assert_eq!(tasks[0].line, 3);
    }

    #[test]
    fn the_body_is_the_exact_text_after_the_checkbox() {
        let doc = Doc::parse("- [ ]   (B) two  spaces  #x   @y  \n");
        let task = &doc.tasks()[0];
        assert_eq!(task.prio, Some(Prio::B));
        assert_eq!(task.body, "(B) two  spaces  #x   @y");
        assert_eq!(task.title, "two spaces");
        assert_eq!(task.tags.len(), 1);
        assert_eq!(task.assignees.len(), 1);
    }

    #[test]
    fn tags_and_assignees_are_found_anywhere_in_the_body() {
        let task = Doc::parse("- [ ] @dave fix you #bug\n").tasks().remove(0);
        assert_eq!(task.title, "fix you");
        assert_eq!(task.tags[0].as_str(), "bug");
        assert_eq!(task.assignees[0].as_str(), "dave");
    }

    #[test]
    fn reads_the_priority_letter_in_either_case() {
        assert_eq!(Doc::parse("- [ ] (a) x\n").tasks()[0].prio, Some(Prio::A));
        assert_eq!(Doc::parse("- [ ] (D) x\n").tasks()[0].prio, Some(Prio::D));
    }

    #[test]
    fn only_a_single_letter_is_a_priority() {
        for line in [
            "- [ ] (A:e2) x\n",
            "- [ ] (p1) x\n",
            "- [ ] (E) x\n",
            "- [ ] (AA) x\n",
        ] {
            let task = Doc::parse(line).tasks().remove(0);
            assert_eq!(task.prio, None, "{line}");
        }
    }

    #[test]
    fn accepts_bullets_ordered_lists_and_uppercase_x() {
        let doc = Doc::parse("* [X] star\n+ [ ] plus\n1. [ ] one\n2) [x] two\n");
        let tasks = doc.tasks();
        assert_eq!(tasks.len(), 4);
        assert!(tasks[0].done);
        assert_eq!(tasks[2].marker, "1.");
        assert_eq!(tasks[3].marker, "2)");
    }

    #[test]
    fn a_line_that_looks_like_a_task_but_is_not_one_is_prose() {
        for line in [
            "- [] x\n",
            "- [ ]\tx\n",
            "  - [x]no space\n",
            "- [y] x\n",
        ] {
            assert!(Doc::parse(line).tasks().is_empty(), "{line:?}");
        }
    }

    #[test]
    fn a_multi_line_title_is_only_its_first_line() {
        let doc = Doc::parse("- [ ] first\n  continued\n");
        assert_eq!(doc.tasks().len(), 1);
        assert_eq!(doc.tasks()[0].title, "first");
    }

    #[test]
    fn an_empty_file_has_no_tasks() {
        assert!(Doc::parse("").tasks().is_empty());
    }
}
