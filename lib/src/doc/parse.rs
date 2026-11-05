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
    pub fn parse_inline(source: &str) -> Self {
        Self {
            lines: lines(source),
            source: Source::Inline,
        }
    }

    /// Every task in the file, in file order.
    pub fn tasks(&self) -> Vec<Task> {
        match self.source {
            Source::Document => scan(&self.lines),
            Source::Inline => scan_inline(&self.lines),
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
fn scan_inline(lines: &[String]) -> Vec<Task> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| comment_line(line, index))
        .collect()
}

/// One line as a comment todo, or `None`.
///
/// The rule is the literal `TODO:` in any case, with at least one character after
/// it that is not a space: a bare `TODO:` promises nothing and is not a todo.
fn comment_line(line: &str, index: usize) -> Option<Task> {
    let lower = line.to_lowercase();
    let at = lower.find("todo:")?;
    let colon = at + "todo:".len();
    // the body starts after the colon and the one space that separates it, if there
    // is one: the rest is the line's, whitespace and all
    let start = match line[colon..].starts_with(' ') {
        true => colon + 1,
        false => colon,
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
    fn an_inline_file_finds_its_todo_comments_and_nothing_else() {
        let doc = Doc::parse_inline(
            "fn main() {\n    // TODO: ship it\n    // FIXME: not this\n    // todo: nor case-blind?\n}\n",
        );
        let tasks = doc.tasks();
        assert_eq!(tasks.len(), 2, "{tasks:#?}");
        assert_eq!(tasks[0].line, 1);
        assert_eq!(tasks[0].title, "ship it");
        assert_eq!(tasks[0].lead(), "    // TODO: ");
        assert_eq!(tasks[1].line, 3);
        assert_eq!(tasks[1].title, "nor case-blind?");
    }

    #[test]
    fn an_inline_todo_needs_something_after_the_colon() {
        assert!(Doc::parse_inline("// TODO:\n// TODO:   \n").tasks().is_empty());
        assert_eq!(Doc::parse_inline("// TODO:x\n").tasks()[0].body, "x");
    }

    #[test]
    fn a_checkbox_in_a_code_file_is_code_and_a_comment_in_markdown_is_prose() {
        assert!(Doc::parse_inline("- [ ] not a todo here\n").tasks().is_empty());
        assert!(Doc::parse("// TODO: not a todo here\n").tasks().is_empty());
    }

    #[test]
    fn an_inline_todo_reads_the_priority_the_tags_and_the_people() {
        let doc = Doc::parse_inline("/* TODO: (A) fix #infra @dave */\n");
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
