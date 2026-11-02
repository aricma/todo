//! Edits: each touches one line and hands back the `Inverse` that undoes it.
//!
//! `check` and `set_prio` edit the line's own bytes rather than rebuilding it,
//! so the spacing, tags and anything unrecognised survive untouched. `set_body`
//! replaces only the body, keeping the bullet, indentation and checkbox.

use super::Doc;
use crate::doc::spans;
use crate::model::{Inverse, Mismatch, Prio, Task};

impl Doc {
    /// Flip the checkbox of one task line.
    pub fn check(&mut self, line: usize) -> Option<Inverse> {
        let task = self.task_at(line)?;
        let at = checkbox_offset(&task);
        let before = self.lines[line].clone();
        let done = if task.done { ' ' } else { 'x' };
        let mut bytes = before.clone().into_bytes();
        bytes[at] = done as u8;
        let after = String::from_utf8(bytes).ok()?;
        self.write(line, before, after)
    }

    /// Set or clear the priority token that opens the body.
    pub fn set_prio(&mut self, line: usize, prio: Option<Prio>) -> Option<Inverse> {
        let task = self.task_at(line)?;
        let body = match (task.prio, prio) {
            (Some(_), Some(prio)) => format!("({}){}", prio.letter(), &task.body[3..]),
            (Some(_), None) => task.body[3..].trim_start().to_owned(),
            (None, Some(prio)) => format!("({}) {}", prio.letter(), task.body),
            (None, None) => return None,
        };
        self.set_body(line, &body)
    }

    /// Replace the body of one task line, keeping everything before it.
    pub fn set_body(&mut self, line: usize, body: &str) -> Option<Inverse> {
        let body = clean(body)?;
        if spans::parts(&body).title.trim().is_empty() {
            return None;
        }
        let task = self.task_at(line)?;
        let before = self.lines[line].clone();
        let after = with_body(&before, &task, &body);
        self.write(line, before, after)
    }

    /// Insert a new unchecked task after `after`, copying its bullet and
    /// indentation. `None` appends at the end of the file.
    pub fn add_after(&mut self, after: Option<usize>, body: &str) -> Option<Inverse> {
        let body = clean(body)?;
        if spans::parts(&body).title.trim().is_empty() {
            return None;
        }
        let (at, indent, marker) = match after.and_then(|line| self.task_at(line)) {
            Some(task) => (task.line + 1, task.indent, task.marker.clone()),
            None => (self.lines.len(), 0, "-".to_owned()),
        };
        let text = format!("{}{} [ ] {body}", " ".repeat(indent), marker);
        self.lines.insert(at, text.clone());
        Some(Inverse::Insert { line: at, text })
    }

    /// Insert a new unchecked task at a line index with an explicit indent,
    /// which is how a nested OPS tree is written back as indented todos.
    pub fn insert_at(&mut self, at: usize, indent: usize, body: &str) -> Option<Inverse> {
        let body = clean(body)?;
        if at > self.lines.len() || spans::parts(&body).title.trim().is_empty() {
            return None;
        }
        let text = format!("{}- [ ] {body}", " ".repeat(indent));
        self.lines.insert(at, text.clone());
        Some(Inverse::Insert { line: at, text })
    }

    /// Remove one task line.
    pub fn delete(&mut self, line: usize) -> Option<Inverse> {
        self.task_at(line)?;
        let text = self.lines.remove(line);
        Some(Inverse::Remove { line, expect: text })
    }

    /// Replay an edit, refusing when the anchored line is not what it was.
    pub fn apply(&mut self, inverse: Inverse) -> Result<(), Mismatch> {
        match inverse {
            Inverse::Set { line, expect, text } => {
                if self.lines.get(line) != Some(&expect) {
                    return Err(Mismatch { line });
                }
                self.lines[line] = text;
                Ok(())
            }
            Inverse::Insert { line, text } => {
                if line > self.lines.len() {
                    return Err(Mismatch { line });
                }
                self.lines.insert(line, text);
                Ok(())
            }
            Inverse::Remove { line, expect } => {
                if self.lines.get(line) != Some(&expect) {
                    return Err(Mismatch { line });
                }
                self.lines.remove(line);
                Ok(())
            }
        }
    }

    /// Record one line replacement as its own inverse.
    fn write(&mut self, line: usize, before: String, after: String) -> Option<Inverse> {
        if before == after {
            return None;
        }
        self.lines[line].clone_from(&after);
        Some(Inverse::Set {
            line,
            expect: before,
            text: after,
        })
    }

    fn task_at(&self, line: usize) -> Option<Task> {
        self.tasks().into_iter().find(|task| task.line == line)
    }
}

/// Refuse text that cannot live on one todo line.
fn clean(body: &str) -> Option<String> {
    if body.contains('\n') || body.contains('\r') {
        return None;
    }
    Some(body.trim().to_owned())
}

/// The byte offset of the checkbox character inside its line.
fn checkbox_offset(task: &Task) -> usize {
    task.indent + task.marker.len() + 2
}

/// The line with its body replaced, keeping the whitespace around the body.
fn with_body(line: &str, task: &Task, body: &str) -> String {
    let start = task.indent + task.marker.len() + 5;
    let raw = &line[start..];
    if raw.trim().is_empty() {
        return format!("{}{body}", &line[..start]);
    }
    let head = &raw[..raw.len() - raw.trim_start().len()];
    let tail = &raw[raw.trim_end().len()..];
    format!("{}{head}{body}{tail}", &line[..start])
}

#[cfg(test)]
mod tests {
    use super::Doc;
    use crate::model::{Inverse, Mismatch, Prio};

    #[test]
    fn check_flips_only_the_checkbox() {
        let mut doc = Doc::parse("- [ ] (A)  two  spaces #x @y  \n");
        assert!(doc.check(0).is_some());
        assert_eq!(doc.render(), "- [x] (A)  two  spaces #x @y  \n");
    }

    #[test]
    fn check_flips_back_through_the_inverse() {
        let source = "- [ ] (A)  two  spaces #x @y  \n";
        let mut doc = Doc::parse(source);
        let inverse = doc.check(0).unwrap();
        doc.apply(inverse.flip()).unwrap();
        assert_eq!(doc.render(), source);
    }

    #[test]
    fn set_prio_writes_the_letter_and_keeps_the_rest() {
        let mut doc = Doc::parse("- [ ] (c) a #x\n");
        assert!(doc.set_prio(0, Some(Prio::C)).is_some());
        assert_eq!(doc.render(), "- [ ] (C) a #x\n");
        assert!(doc.set_prio(0, None).is_some());
        assert_eq!(doc.render(), "- [ ] a #x\n");
        assert!(doc.set_prio(0, Some(Prio::A)).is_some());
        assert_eq!(doc.render(), "- [ ] (A) a #x\n");
    }

    #[test]
    fn set_prio_leaves_a_non_priority_token_as_text() {
        let mut doc = Doc::parse("- [ ] (p1) a\n");
        assert!(doc.set_prio(0, Some(Prio::B)).is_some());
        assert_eq!(doc.render(), "- [ ] (B) (p1) a\n");
    }

    #[test]
    fn set_prio_preserves_the_spaces_around_the_body() {
        let mut doc = Doc::parse("- [ ]  (A)   x   #t  \n");
        assert!(doc.set_prio(0, Some(Prio::D)).is_some());
        assert_eq!(doc.render(), "- [ ]  (D)   x   #t  \n");
    }

    #[test]
    fn setting_the_same_priority_changes_nothing_but_clearing_it_does() {
        let mut doc = Doc::parse("- [ ] (A) a\n");
        assert_eq!(doc.set_prio(0, Some(Prio::A)), None);
        assert!(doc.set_prio(0, None).is_some());
        assert_eq!(doc.render(), "- [ ] a\n");
        assert_eq!(doc.set_prio(0, None), None);
    }

    #[test]
    fn set_body_replaces_the_body_and_keeps_the_prefix() {
        let mut doc = Doc::parse("  * [x] old #a\n");
        assert!(doc.set_body(0, "new #t @u").is_some());
        assert_eq!(doc.render(), "  * [x] new #t @u\n");
    }

    #[test]
    fn set_body_refuses_an_empty_title_or_a_newline() {
        let source = "- [ ] keep me\n";
        let mut doc = Doc::parse(source);
        for body in ["", "   ", "@x", "#x @y", "\n", "a\nb", "a\rb"] {
            assert_eq!(doc.set_body(0, body), None, "{body:?}");
        }
        assert_eq!(doc.render(), source);
    }

    #[test]
    fn set_body_keeps_unrecognised_tokens_as_text() {
        let mut doc = Doc::parse("- [ ] old\n");
        doc.set_body(0, "x #café @a b").unwrap();
        assert_eq!(doc.render(), "- [ ] x #café @a b\n");
    }

    #[test]
    fn delete_removes_one_line_and_the_inverse_puts_it_back() {
        let source = "- [ ] a\n- [ ] b\n- [ ] c\n";
        let mut doc = Doc::parse(source);
        let inverse = doc.delete(1).unwrap();
        assert_eq!(doc.render(), "- [ ] a\n- [ ] c\n");
        doc.apply(inverse.flip()).unwrap();
        assert_eq!(doc.render(), source);
    }

    #[test]
    fn add_after_copies_the_bullet_and_indent_of_its_neighbour() {
        let mut doc = Doc::parse("## x\n  * [ ] child\n");
        assert!(doc.add_after(Some(1), "new #t").is_some());
        assert_eq!(doc.render(), "## x\n  * [ ] child\n  * [ ] new #t\n");
    }

    #[test]
    fn add_after_none_appends_a_dash_at_the_end() {
        let mut doc = Doc::parse("- [ ] a\n");
        let inverse = doc.add_after(None, "last @sam").unwrap();
        assert_eq!(doc.render(), "- [ ] a\n- [ ] last @sam\n");
        assert_eq!(
            inverse,
            Inverse::Insert {
                line: 1,
                text: "- [ ] last @sam".into()
            }
        );
    }

    #[test]
    fn add_after_trims_and_refuses_a_bodiless_task() {
        let mut doc = Doc::parse("- [ ] a\n");
        assert!(doc.add_after(None, "  spaced  ").is_some());
        assert_eq!(doc.render(), "- [ ] a\n- [ ] spaced\n");
        assert_eq!(doc.add_after(None, "   "), None);
        assert_eq!(doc.add_after(None, "@only"), None);
    }

    #[test]
    fn apply_refuses_when_the_anchored_line_moved() {
        let mut doc = Doc::parse("- [x] someone else\n");
        let stale = Inverse::Set {
            line: 0,
            expect: "- [ ] a".into(),
            text: "- [x] a".into(),
        };
        assert_eq!(doc.apply(stale), Err(Mismatch { line: 0 }));
        assert_eq!(doc.render(), "- [x] someone else\n");
    }

    #[test]
    fn apply_refuses_an_insert_past_the_end() {
        let mut doc = Doc::parse("- [ ] a\n");
        let insert = Inverse::Insert {
            line: 9,
            text: "- [ ] b".into(),
        };
        assert_eq!(doc.apply(insert), Err(Mismatch { line: 9 }));
    }

    #[test]
    fn edits_leave_a_prose_line_untouched() {
        let source = "just prose  \n";
        let mut doc = Doc::parse(source);
        assert_eq!(doc.check(0), None);
        assert_eq!(doc.set_prio(0, Some(Prio::A)), None);
        assert_eq!(doc.set_body(0, "x"), None);
        assert_eq!(doc.delete(0), None);
        assert_eq!(doc.render(), source);
    }
}
