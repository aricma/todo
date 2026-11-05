//! `--check` findings and `--format`, both about todo rules only.
//!
//! The three findings are: a line that looks like a task but is not one, a task
//! with nothing left for a title, and a body that opens with a priority token
//! whose letter is not A–D. Everything else — spacing, `[X]`, trailing
//! whitespace, an unrecognised `#token` — is out of scope by design, and
//! `format` fixes only the case of the checkbox and of the priority letter.

use super::{parse, Doc};
use crate::model::Finding;

impl Doc {
    /// Every todo-rule violation, in line order.
    pub fn findings(&self) -> Vec<Finding> {
        // the rules are about todo documents; a comment todo has none to break
        if self.source == super::Source::Inline {
            return Vec::new();
        }
        let fenced = parse::fenced(&self.lines);
        let mut findings = Vec::new();
        for (index, line) in self.lines.iter().enumerate() {
            if fenced[index] {
                continue;
            }
            match parse::parse_line(line, index) {
                Some(task) => {
                    if task.title.trim().is_empty() {
                        findings.push(Finding {
                            line: index,
                            message: format!("no title: \"{}\"", line.trim()),
                        });
                    }
                    if let Some(letter) = rejected_priority(&task.body) {
                        findings.push(Finding {
                            line: index,
                            message: format!("priority must be A-D, not ({letter})"),
                        });
                    }
                }
                None if looks_like_task(line) => findings.push(Finding {
                    line: index,
                    message: format!("malformed task line: \"{}\"", line.trim()),
                }),
                None => {}
            }
        }
        findings
    }
}

/// A line that meant to be a task: bullet, space, then `[x]` with one char
/// between the brackets. A markdown link item (`- [docs](url)`) is not one.
fn looks_like_task(line: &str) -> bool {
    let trimmed = line.trim_start_matches(' ');
    let Some((_, after)) = parse::bullet(trimmed) else {
        return false;
    };
    let Some(after) = after.strip_prefix(' ') else {
        return false;
    };
    let Some(rest) = after.strip_prefix('[') else {
        return false;
    };
    let mut characters = rest.chars();
    matches!((characters.next(), characters.next()), (Some(_), Some(']')))
}

/// The letter of a `(X)` token opening the body, when it is not A–D.
fn rejected_priority(body: &str) -> Option<char> {
    let inner = body.strip_prefix('(')?;
    let close = inner.find(')')?;
    let token = &inner[..close];
    let mut characters = token.chars();
    let (Some(letter), None) = (characters.next(), characters.next()) else {
        return None;
    };
    if crate::model::Prio::from_letter(letter).is_some() {
        None
    } else {
        Some(letter)
    }
}

#[cfg(test)]
mod tests {
    use super::Doc;

    fn messages(doc: &Doc) -> Vec<String> {
        doc.findings()
            .into_iter()
            .map(|finding| format!("{}: {}", finding.line, finding.message))
            .collect()
    }

    #[test]
    fn findings_report_the_three_rules() {
        let doc = Doc::parse("- [ ]no space\n- [ ] (A) #x @y\n- [ ] (E) x\n");
        let found = messages(&doc);
        assert_eq!(found.len(), 3, "{found:#?}");
        assert!(
            found[0].starts_with("0: malformed task line"),
            "{}",
            found[0]
        );
        assert!(found[1].starts_with("1: no title"), "{}", found[1]);
        assert!(
            found[2].starts_with("2: priority must be A-D"),
            "{}",
            found[2]
        );
    }

    #[test]
    fn findings_ignore_formatting_and_unknown_tokens() {
        let doc =
            Doc::parse("- [X] fine   \n- [ ] a @x #y\n- [ ] #café\n- [ ] (p1) x\n- [ ] (A:e2) x\n");
        assert_eq!(messages(&doc), Vec::<String>::new());
    }

    #[test]
    fn a_markdown_link_item_is_not_a_malformed_task() {
        let doc = Doc::parse("- [docs](https://example.test)\n- [ ] fine\n");
        assert_eq!(messages(&doc), Vec::<String>::new());
    }

    #[test]
    fn a_fenced_example_is_never_a_finding() {
        let doc = Doc::parse("```\n- [ ]no space\n- [ ] (E) x\n```\n");
        assert_eq!(messages(&doc), Vec::<String>::new());
    }

    #[test]
    fn format_fixes_only_checkbox_and_letter_case() {
        let mut doc = Doc::parse("- [X] (b) a  #x  \n  * [X] (d) b\n- [ ] (A) c\n");
        doc.format();
        assert_eq!(
            doc.render(),
            "- [x] (B) a  #x  \n  * [x] (D) b\n- [ ] (A) c\n"
        );
        assert_eq!(messages(&doc), Vec::<String>::new());
    }

    #[test]
    fn format_is_idempotent_and_never_touches_non_task_lines() {
        let mut doc = Doc::parse("para  \n- [X] a\n");
        doc.format();
        let once = doc.render();
        doc.format();
        assert_eq!(doc.render(), once);
        assert_eq!(once, "para  \n- [x] a\n");
    }

    #[test]
    fn format_leaves_unknown_tokens_and_fences_alone() {
        let source = "```\n- [X] (b) inside\n```\n- [ ] a #café (p1) @x #y\n";
        let mut doc = Doc::parse(source);
        doc.format();
        assert_eq!(
            doc.render(),
            "```\n- [X] (b) inside\n```\n- [ ] a #café (p1) @x #y\n"
        );
    }
}
