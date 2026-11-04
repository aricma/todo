//! Edits: each touches one line and hands back the `Inverse` that undoes it.
//!
//! `check` and `set_prio` edit the line's own bytes rather than rebuilding it,
//! so the spacing, tags and anything unrecognised survive untouched. `set_body`
//! replaces only the body, keeping the bullet, indentation and checkbox.

use super::Doc;
use crate::doc::{headings, parse, spans};
use crate::model::{Blocker, Inverse, Mismatch, Prio, Task};

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

    /// Add a heading at the end of the file.
    ///
    /// The text carries its own level: `## later` stays a second-level heading, and
    /// plain `later` becomes `# later`. A file with no headings gets its first one
    /// this way, which is how the outline stops being empty.
    pub fn add_heading(&mut self, text: &str) -> Option<Inverse> {
        let line = heading_line(text)?;
        let at = self.lines.len();
        self.lines.push(line.clone());
        Some(Inverse::Insert {
            line: at,
            text: line,
        })
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

    /// Exchange a task line with the nearest task line above or below it.
    ///
    /// A heading is a wall: a todo belongs to the section it is written under, so
    /// moving one past `#` would file it somewhere nobody asked for. Prose and
    /// blank lines are not walls — the todo steps over them and they stay put,
    /// because moving a note with it would change what the note is about.
    pub fn move_task(&mut self, line: usize, up: bool) -> Result<Inverse, Blocker> {
        self.task_at(line).ok_or(Blocker::Edge)?;
        let fenced = parse::fenced(&self.lines);
        let mut at = line;
        let other = loop {
            at = if up {
                at.checked_sub(1).ok_or(Blocker::Edge)?
            } else {
                at.checked_add(1).ok_or(Blocker::Edge)?
            };
            let Some(line) = self.lines.get(at) else {
                return Err(Blocker::Edge);
            };
            if *fenced.get(at).unwrap_or(&false) {
                continue;
            }
            if headings::heading(line, at).is_some() {
                return Err(Blocker::Heading);
            }
            if self.task_at(at).is_some() {
                break at;
            }
        };
        let (a, b) = if line < other {
            (line, other)
        } else {
            (other, line)
        };
        let expect_a = self.lines[a].clone();
        let expect_b = self.lines[b].clone();
        self.lines.swap(a, b);
        Ok(Inverse::Swap {
            a,
            b,
            expect_a,
            expect_b,
        })
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
            Inverse::Swap {
                a,
                b,
                expect_a,
                expect_b,
            } => {
                if self.lines.get(a) != Some(&expect_a) || self.lines.get(b) != Some(&expect_b) {
                    return Err(Mismatch { line: a });
                }
                self.lines.swap(a, b);
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

/// A heading line from what was typed: the hashes if there are any, one `#`
/// otherwise, and the text after them trimmed.
fn heading_line(text: &str) -> Option<String> {
    let text = clean(text)?;
    let level = text
        .chars()
        .take_while(|character| *character == '#')
        .count();
    let (level, body) = match level {
        0 => (1, text.as_str()),
        1..=6 => (level, text[level..].trim_start()),
        _ => return None,
    };
    if body.trim().is_empty() {
        return None;
    }
    Some(format!("{} {}", "#".repeat(level), body.trim_end()))
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
    #[test]
    fn a_heading_carries_its_own_level_and_defaults_to_one() {
        for (typed, expected) in [
            ("later", "# later"),
            ("## later", "## later"),
            ("  ### deeper  ", "### deeper"),
            ("# spaced   ", "# spaced"),
        ] {
            let mut doc = Doc::parse("- [ ] a\n");
            let inverse = doc.add_heading(typed).expect("a heading");
            assert_eq!(doc.render(), format!("- [ ] a\n{expected}\n"), "{typed:?}");
            // and it undoes like any other insert
            doc.apply(inverse.flip()).expect("undo");
            assert_eq!(doc.render(), "- [ ] a\n", "{typed:?}");
        }
    }

    #[test]
    fn a_heading_with_no_text_is_refused() {
        let mut doc = Doc::parse("");
        assert!(doc.add_heading("").is_none());
        assert!(doc.add_heading("   ").is_none());
        assert!(doc.add_heading("#").is_none());
        assert!(doc.add_heading("####### seven").is_none());
        assert_eq!(doc.render(), "");
    }

    use super::Doc;
    use crate::model::Blocker;
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
    fn a_todo_moves_past_its_neighbour_and_the_inverse_puts_it_back() {
        let source = "- [ ] a\n- [ ] b\n- [ ] c\n";
        let mut doc = Doc::parse(source);
        let inverse = doc.move_task(0, false).unwrap();
        assert_eq!(doc.render(), "- [ ] b\n- [ ] a\n- [ ] c\n");
        doc.apply(inverse.flip()).unwrap();
        assert_eq!(doc.render(), source);
    }

    #[test]
    fn a_move_steps_over_prose_and_blank_lines_but_never_a_heading() {
        let mut doc = Doc::parse("- [ ] a\n\nnote about a\n\n- [ ] b\n");
        doc.move_task(0, false).unwrap();
        // `b` is now first, and the note has not been dragged anywhere
        assert_eq!(doc.render(), "- [ ] b\n\nnote about a\n\n- [ ] a\n");

        let source = "## one\n- [ ] a\n\n## two\n- [ ] b\n";
        let mut doc = Doc::parse(source);
        assert_eq!(
            doc.move_task(1, false),
            Err(Blocker::Heading),
            "the heading is a wall"
        );
        assert_eq!(doc.render(), source);
    }

    #[test]
    fn a_move_at_the_top_or_the_bottom_of_the_file_does_nothing() {
        let source = "- [ ] only\n";
        let mut doc = Doc::parse(source);
        assert_eq!(doc.move_task(0, true), Err(Blocker::Edge));
        assert_eq!(doc.move_task(0, false), Err(Blocker::Edge));
        assert_eq!(doc.render(), source);
    }

    #[test]
    fn a_move_only_touches_task_lines() {
        let source = "prose\n- [ ] a\n";
        let mut doc = Doc::parse(source);
        assert_eq!(
            doc.move_task(0, true),
            Err(Blocker::Edge),
            "prose is not a task"
        );
        assert_eq!(doc.render(), source);
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
