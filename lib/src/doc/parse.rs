//! Parsing: a file becomes exact lines, and a line becomes a `Task`.
//!
//! `scan` is the single place that decides what counts as a task, so `tasks`,
//! `findings` and `format` can never disagree about it.

use super::comment::{self, Comment};
use super::spans;
use super::{Doc, Source, Syntax};
use crate::model::{Assignee, Severity, Tag, Task};

impl Doc {
    /// Split a file into lines. CRLF loses its `\r`, a BOM is dropped, and the
    /// trailing newline is not a line of its own.
    pub fn parse(source: &str) -> Self {
        Self {
            lines: lines(source),
            source: Source::Document,
            syntax: Syntax::Any,
        }
    }

    /// The same, for a file whose todos are todo comments: any file in the tree that
    /// is not a todo document. Its lines are kept exactly as they are, so the code
    /// around a todo is neither moved nor rewritten (RFC-016).
    ///
    /// The comment markers are [`Syntax::Any`], which is what a caller that does not
    /// know the file's name can honestly claim — a file read with `parse_at` gets its
    /// own language's markers (RFC-020).
    pub fn parse_comment(source: &str) -> Self {
        Self::parse_comment_in(Syntax::Any, source)
    }

    /// The same, with the markers of a known language.
    pub fn parse_comment_in(syntax: Syntax, source: &str) -> Self {
        Self {
            lines: lines(source),
            source: Source::Comment,
            syntax,
        }
    }

    /// Every task in the file, in file order.
    pub fn tasks(&self) -> Vec<Task> {
        match self.source {
            Source::Document => scan(&self.lines),
            Source::Comment => scan_comment(&self.lines, self.syntax),
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

/// Walk the lines of a code file and collect its todo comments.
///
/// No fences and no headings: a code file is not markdown, and `# TODO:` in a shell
/// script is a comment, not an outline. One todo per line, the first severity word on
/// it that passes the rule, and the code around it is the todo's lead (RFC-016).
fn scan_comment(lines: &[String], syntax: Syntax) -> Vec<Task> {
    comment::comments(lines, syntax)
        .iter()
        .filter_map(|comment| todo_in(&lines[comment.line], comment))
        .collect()
}

/// The todo this comment holds, or `None` — the second half of reading a comment
/// todo, and where RFC-019's rule now lives.
///
/// The comment is the region, and the word may be anywhere in it: `// note: FIXME: x`
/// is a todo, because both halves are inside the comment. What still has to hold is
/// the rule itself, one word at a time:
///
/// - a severity word in capitals, followed by its colon — or by an assignee in
///   parentheses and then its colon: `TODO(alice): ` (RFC-020, RFC-021);
/// - the word has to start a word — `NOTTODO:` is a longer word, not a todo;
/// - nothing between the start of the line and the word may be a quote, which is the
///   only thing standing between this and a `TODO:` inside a string (RFC-019);
/// - at least one character after the colon that is not a space, because a bare
///   `TODO:` promises something and says nothing.
fn todo_in(line: &str, comment: &Comment) -> Option<Task> {
    let word = words(&line[comment.at..])
        .into_iter()
        .map(|word| Word {
            at: comment.at + word.at,
            after: comment.at + word.after,
            ..word
        })
        .find(|word| at_word_start(line, word.at) && !quoted(&line[..word.at]))?;
    // the body starts after the colon and the one space that separates it, if there
    // is one: the rest is the line's, whitespace and all
    let start = if line[word.after..].starts_with(' ') {
        word.after + 1
    } else {
        word.after
    };
    let body = line[start..].trim();
    if body.is_empty() {
        return None;
    }
    let parts = spans::parts(body);
    // what the word filed is in the same list the body's own `#tag`/`@person` goes in,
    // once: a todo is one thing with one set of people and one set of tags on it,
    // however the line spells them (RFC-021)
    let mut tags = parts.tags;
    let mut assignees = parts.assignees;
    match word.filed {
        Some(Filed::Tag(tag)) if !tags.contains(&tag) => tags.insert(0, tag),
        Some(Filed::Person(who)) if !assignees.contains(&who) => assignees.insert(0, who),
        _ => {}
    }
    Some(Task {
        line: comment.line,
        indent: line.len() - line.trim_start_matches(' ').len(),
        done: false,
        severity: Some(word.severity),
        prio: parts.prio,
        title: parts.title,
        body: body.to_owned(),
        tags,
        assignees,
        marker: String::new(),
        // everything the tool does not own, the parentheses included
        prefix: Some(line[..start].to_owned()),
    })
}

/// One severity word with its colon, and what a comment may put between them.
struct Word {
    /// Byte offset of the word in the text it was found in.
    at: usize,
    severity: Severity,
    /// The `(@alice)` or `(#home)` of `TODO(@alice): ` — one of the two things a body
    /// already has, and never a priority: a priority is written after the colon, where
    /// the body is (RFC-021).
    filed: Option<Filed>,
    /// Byte offset just after the colon.
    after: usize,
}

/// A parenthesis in front of the colon can name a person or file the todo under a tag.
///
/// Two boxes, not a third kind of field: whatever is inside is already one of the body's
/// own tokens, or it is nothing (RFC-021).
#[derive(Clone, Debug, PartialEq, Eq)]
enum Filed {
    Person(Assignee),
    Tag(Tag),
}

/// The `(@alice)` of `TODO(@alice): `, what it is, and how many bytes it takes.
///
/// `None` when what follows the word is neither `:` nor `(…):` holding a `@person` or a
/// `#tag`. A bare word is not accepted — `TODO(alice): ` and `TODO(bob): ` do not say
/// whether they name a person or file the line, and the sigil is the one character that
/// does. `TODO (x): ` (space), a parenthesis that never closes, and `TODO(): ` are not a
/// word either.
fn in_parens(text: &str) -> Option<(Filed, usize)> {
    let rest = text.strip_prefix('(')?;
    let (word, after) = rest.split_once(')')?;
    let after = after.strip_prefix(':')?;
    let filed = match (Assignee::parse(word), Tag::parse(word)) {
        (Some(person), _) => Filed::Person(person),
        (None, Some(tag)) => Filed::Tag(tag),
        (None, None) => return None,
    };
    Some((filed, text.len() - after.len()))
}

/// Every severity word that opens a todo, earliest first.
fn words(text: &str) -> Vec<Word> {
    let mut found: Vec<Word> = Vec::new();
    for severity in Severity::ALL {
        for (at, _) in text.match_indices(severity.word()) {
            let rest = &text[at + severity.word().len()..];
            if severity.opens(&text[at..]) {
                found.push(Word {
                    at,
                    severity,
                    filed: None,
                    after: at + severity.word().len() + 1,
                });
            } else if let Some((filed, taken)) = in_parens(rest) {
                found.push(Word {
                    at,
                    severity,
                    filed: Some(filed),
                    after: at + severity.word().len() + taken,
                });
            }
        }
    }
    found.sort_by_key(|word| word.at);
    found
}

/// Whether the byte at `at` starts a word: `NOTTODO:` is not a todo, it is a longer
/// word that happens to end in one.
fn at_word_start(text: &str, at: usize) -> bool {
    !text[..at]
        .chars()
        .next_back()
        .is_some_and(|character| character.is_alphanumeric() || character == '_')
}

/// Whether a quote stands in the text in front of a word.
///
/// This is the heuristic in place of knowing a language's strings, and it is what the
/// rule costs: `let a = "x"; // TODO: pay rent` is a todo the tool cannot see, because
/// it cannot tell which side of the quote it is on (RFC-019).
fn quoted(lead: &str) -> bool {
    lead.chars().any(|c| matches!(c, '"' | '\'' | '`'))
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
        severity: None,
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
    use crate::model::{Prio, Severity};

    #[test]
    fn a_comment_todo_file_finds_its_todo_comments_and_nothing_else() {
        let doc = Doc::parse_comment(
            "fn main() {\n    // TODO: ship it\n    // FIXME: nor this\n    // todo: nor this\n}\n",
        );
        let tasks = doc.tasks();
        // `FIXME:` and `TODO:` are both todos, and the lowercase word is not (RFC-020)
        assert_eq!(tasks.len(), 2, "{tasks:#?}");
        assert_eq!(tasks[0].line, 1);
        assert_eq!(tasks[0].title, "ship it");
        assert_eq!(tasks[0].severity, Some(Severity::Todo));
        assert_eq!(tasks[0].lead(), "    // TODO: ");
        assert_eq!(tasks[1].severity, Some(Severity::Fixme));
        assert_eq!(tasks[1].lead(), "    // FIXME: ");
    }

    #[test]
    fn every_severity_word_opens_a_todo_and_says_which_it_is() {
        let doc = Doc::parse_comment("// TODO: a\n// FIXME: b\n// HACK: c\n// XXX: d\n");
        let tasks = doc.tasks();
        assert_eq!(tasks.len(), 4, "{tasks:#?}");
        assert_eq!(
            tasks
                .iter()
                .map(|task| task.severity.unwrap().word())
                .collect::<Vec<_>>(),
            ["TODO", "FIXME", "HACK", "XXX"]
        );
        // the word stays in the line: the tool does not rewrite the comment, and a
        // `FIXME:` reads as one on the screen without the model rendering it
        assert_eq!(tasks[1].lead(), "// FIXME: ");
        assert_eq!(tasks[1].body, "b");
        assert_eq!(tasks[1].render(), "// FIXME: b");
    }

    #[test]
    fn a_word_may_name_who_it_is_for() {
        let doc = Doc::parse_comment("// FIXME(@alice): the retry is wrong\n");
        let task = &doc.tasks()[0];
        // the person is in the same list the body's `@alice` would go in, and the text
        // stays in the lead: the tool never rewrites a comment's own grammar
        assert_eq!(
            task.assignees
                .iter()
                .map(|who| who.as_str().to_owned())
                .collect::<Vec<_>>(),
            ["alice"]
        );
        assert_eq!(task.lead(), "// FIXME(@alice): ");
        assert_eq!(task.body, "the retry is wrong");
        assert_eq!(task.render(), "// FIXME(@alice): the retry is wrong");
        assert_eq!(task.title, "the retry is wrong");
    }

    #[test]
    fn a_word_may_file_it_under_a_tag() {
        let doc = Doc::parse_comment("// TODO(#home): pay rent\n");
        let task = &doc.tasks()[0];
        // the tag is a tag, in the body's own list, and `#102` is one too: a number in a
        // tag is a tag (RFC-021)
        assert_eq!(
            task.tags
                .iter()
                .map(|tag| tag.as_str().to_owned())
                .collect::<Vec<_>>(),
            ["home"]
        );
        assert_eq!(task.lead(), "// TODO(#home): ");
        assert_eq!(task.title, "pay rent");
        let number = &Doc::parse_comment("// TODO: #102 pay rent\n").tasks()[0];
        assert_eq!(number.tags[0].as_str(), "102");
    }

    #[test]
    fn what_the_word_files_is_in_the_body_s_own_list() {
        let doc = Doc::parse_comment("// TODO(@alice): ship it @bob #home\n");
        let task = &doc.tasks()[0];
        assert_eq!(
            task.assignees
                .iter()
                .map(|who| who.as_str().to_owned())
                .collect::<Vec<_>>(),
            ["alice", "bob"]
        );
        assert_eq!(
            task.tags
                .iter()
                .map(|tag| tag.as_str().to_owned())
                .collect::<Vec<_>>(),
            ["home"]
        );
        // and naming one person twice is still one person
        let twice = Doc::parse_comment("// TODO(@alice): ship it @alice\n");
        assert_eq!(twice.tasks()[0].assignees.len(), 1);
    }

    #[test]
    fn a_parenthesis_without_a_sigil_is_not_a_word() {
        // a bare word says nothing about which of the two it is, so the line is not a
        // todo at all — and neither is one with a priority where the sigil would be
        for line in [
            "// TODO(alice): pay rent\n",
            "// TODO(bob): pay rent\n",
            "// TODO(102): pay rent\n",
            "// TODO(A): pay rent\n",
        ] {
            assert!(Doc::parse_comment(line).tasks().is_empty(), "{line:?}");
        }
        // and the priority is still read where the grammar puts it
        let task = &Doc::parse_comment("// TODO(@alice): (A) pay rent\n").tasks()[0];
        assert_eq!(task.prio, Some(Prio::A));
        assert_eq!(task.assignees[0].as_str(), "alice");
    }

    #[test]
    fn a_marker_in_the_middle_of_a_comment_still_counts() {
        // the word is not tied to the marker right in front of it: everything inside
        // the comment is the comment (RFC-020)
        let doc = Doc::parse_comment("// note: FIXME: the retry is wrong\n");
        let tasks = doc.tasks();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].body, "the retry is wrong");
        assert_eq!(tasks[0].lead(), "// note: FIXME: ");
    }

    #[test]
    fn the_severity_word_has_to_start_a_word() {
        for line in ["// NOTTODO: x\n", "// A_TODO: x\n", "// XFIXME: x\n"] {
            assert!(Doc::parse_comment(line).tasks().is_empty(), "{line:?}");
        }
    }

    #[test]
    fn only_the_marker_this_language_uses_opens_a_comment() {
        use crate::doc::Syntax;
        let rust = Syntax::of(std::path::Path::new("src/main.rs"));
        // `#` is an attribute in rust, and a comment in shell
        assert!(
            Doc::parse_comment_in(rust, "#[derive(Debug)] // TODO: x\n")
                .tasks()
                .len()
                == 1
        );
        assert!(Doc::parse_comment_in(rust, "# TODO: x\n")
            .tasks()
            .is_empty());
        let shell = Syntax::of(std::path::Path::new("run.sh"));
        assert_eq!(Doc::parse_comment_in(shell, "# TODO: x\n").tasks().len(), 1);
        assert!(Doc::parse_comment_in(shell, "// TODO: x\n")
            .tasks()
            .is_empty());
        // a file the tool does not know keeps every marker, and a bare line
        assert_eq!(Doc::parse_comment("// TODO: x\n").tasks().len(), 1);
        assert_eq!(Doc::parse_comment("TODO: x\n").tasks().len(), 1);
    }

    #[test]
    fn a_comment_todo_needs_something_after_the_colon() {
        assert!(Doc::parse_comment("// TODO:\n// TODO:   \n")
            .tasks()
            .is_empty());
        assert_eq!(Doc::parse_comment("// TODO:x\n").tasks()[0].body, "x");
    }

    #[test]
    fn a_checkbox_in_a_code_file_is_code_and_a_comment_in_markdown_is_prose() {
        assert!(Doc::parse_comment("- [ ] not a todo here\n")
            .tasks()
            .is_empty());
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
        assert!(
            Doc::parse_comment("// a file with no todo: the bar says less\n")
                .tasks()
                .is_empty()
        );
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
        assert_eq!(
            Doc::parse_comment("TODO: plain\n  TODO: indented\n")
                .tasks()
                .len(),
            2
        );
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
            // every severity word, since RFC-020
            "// FIXME: pay rent",
            "// HACK: pay rent",
            "// XXX: pay rent",
            // a person or a tag between the word and its colon, since RFC-021
            "// TODO(@bob): pay rent",
            "// FIXME(#home): pay rent",
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
            // the parenthesis rule: no sigil says nothing, and a word has to be closed
            "// TODO(bob): pay rent",
            "// TODO(A): pay rent",
            "// TODO(): pay rent",
            "// TODO(bob pay rent",
            "// TODO (@bob): pay rent",
            // the word rule: a severity that is the tail of a longer word
            "// NOTTODO: pay rent",
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
            Doc::parse_comment("/// we use <!-- TODO: --> in prose\n")
                .tasks()
                .len(),
            1
        );
    }

    #[test]
    fn a_string_before_a_trailing_comment_hides_it() {
        // the cost of the quote rule: a note after code that quotes something is
        // missed, because the tool cannot tell which side of the quote it is on
        assert!(Doc::parse_comment("let a = \"x\"; // TODO: this\n")
            .tasks()
            .is_empty());
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
        for line in ["- [] x\n", "- [ ]\tx\n", "  - [x]no space\n", "- [y] x\n"] {
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
