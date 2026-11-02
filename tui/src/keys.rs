//! Keys to state: the whole key map in one place.

use aricma_todo_lib::model::{Prio, Task};
use ratatui::crossterm::event::KeyCode;

use super::action::{Action, Pane};
use super::app::{shift, App};
use super::prompt::{Mode, Prompt};

impl App {
    /// Handle one key press. Anything that touches the disk comes back as an
    /// `Action` for the caller.
    pub fn on_key(&mut self, code: KeyCode) -> Option<Action> {
        match self.mode.clone() {
            Mode::Adding(prompt) => self.typing(prompt, true, code),
            Mode::Editing(prompt) => self.typing(prompt, false, code),
            Mode::Confirming => self.confirming(code),
            Mode::Normal => self.normal(code),
        }
    }

    /// A key while a prompt is open: type, move, commit, cancel.
    fn typing(&mut self, mut prompt: Prompt, adding: bool, code: KeyCode) -> Option<Action> {
        match code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                return None;
            }
            KeyCode::Enter => {
                self.mode = Mode::Normal;
                let text = prompt.text().to_owned();
                return if adding {
                    self.add(&text)
                } else {
                    self.edit(&text)
                };
            }
            KeyCode::Left => prompt.left(),
            KeyCode::Right => prompt.right(),
            KeyCode::Home => prompt.home(),
            KeyCode::End => prompt.end(),
            KeyCode::Backspace => prompt.backspace(),
            KeyCode::Delete => prompt.delete(),
            KeyCode::Char(character) => prompt.insert(character),
            _ => {}
        }
        self.mode = if adding {
            Mode::Adding(prompt)
        } else {
            Mode::Editing(prompt)
        };
        None
    }

    fn confirming(&mut self, code: KeyCode) -> Option<Action> {
        self.mode = Mode::Normal;
        match code {
            KeyCode::Char('y' | 'Y') => {
                let task = self.current_task()?;
                let inverse = self.doc.delete(task.line)?;
                self.clamp_task();
                Some(self.record(label("delete", &task), inverse))
            }
            _ => None,
        }
    }

    fn normal(&mut self, code: KeyCode) -> Option<Action> {
        match code {
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char('r') => Some(Action::Reload),
            // tab loops the sidebar; the sideways arrows cross to the file view
            KeyCode::Tab => {
                self.focus(true);
                None
            }
            KeyCode::BackTab => {
                self.focus(false);
                None
            }
            KeyCode::Right => {
                self.enter_view();
                None
            }
            KeyCode::Left => {
                self.leave_view();
                None
            }
            KeyCode::Down => self.step(1),
            KeyCode::Up => self.step(-1),
            KeyCode::Char('g') => self.jump(false),
            KeyCode::Char('G') => self.jump(true),
            KeyCode::Char('x' | ' ') => self.toggle(),
            KeyCode::Char('d') => self.ask_delete(),
            KeyCode::Char('a') => self.ask_add(),
            KeyCode::Char('e') => self.ask_edit(),
            KeyCode::Char('0') => self.priority(None),
            KeyCode::Char(letter @ 'A'..='D') => {
                let prio = Prio::from_letter(letter)?;
                self.priority(Some(prio))
            }
            KeyCode::Esc => {
                self.message = None;
                None
            }
            _ => None,
        }
    }

    /// Move the focus to the next or previous sidebar section, looping. From the
    /// file view, `tab` goes back to the sidebar rather than staying on the right.
    pub fn focus(&mut self, forward: bool) {
        if let Some(pane) = self.focus_target(forward) {
            self.pane = pane;
        }
    }

    /// Move one row inside the focused section.
    fn step(&mut self, delta: isize) -> Option<Action> {
        match self.pane {
            Pane::Files => {
                if let Err(error) = self.select_file(delta) {
                    self.set_message(error.to_string());
                }
                self.sync_outline();
            }
            Pane::Outline => {
                self.outline_idx = shift(self.outline_idx, delta, self.headings().len());
                self.jump_from_outline();
            }
            Pane::View | Pane::Todos => {
                self.todo(delta);
            }
            Pane::History => {
                self.history_idx = shift(self.history_idx, delta, self.file_history().len());
            }
        }
        None
    }

    /// Move the selected todo, from any section, and take the outline with it.
    fn todo(&mut self, delta: isize) -> Option<Action> {
        self.task_idx = shift(self.task_idx, delta, self.tasks().len());
        self.sync_outline();
        None
    }

    fn jump(&mut self, last: bool) -> Option<Action> {
        match self.pane {
            Pane::Files => {
                let target = if last {
                    self.files.len().saturating_sub(1)
                } else {
                    0
                };
                let delta = i64::try_from(target).unwrap_or(i64::MAX)
                    - i64::try_from(self.file_idx).unwrap_or(0);
                let delta = isize::try_from(delta).unwrap_or(isize::MAX);
                if let Err(error) = self.select_file(delta) {
                    self.set_message(error.to_string());
                }
            }
            Pane::Outline => {
                let count = self.headings().len();
                self.outline_idx = if last { count.saturating_sub(1) } else { 0 };
                self.jump_from_outline();
            }
            Pane::View | Pane::Todos => {
                let count = self.tasks().len();
                self.task_idx = if last { count.saturating_sub(1) } else { 0 };
                self.sync_outline();
            }
            Pane::History => {
                let count = self.file_history().len();
                self.history_idx = if last { count.saturating_sub(1) } else { 0 };
            }
        }
        None
    }

    /// `x`: check a todo, or uncheck a history entry.
    ///
    /// Checking belongs to the panel that shows the todo: the file view (or the
    /// todos list of a single-file session) and nowhere else. In the files and
    /// outline sections `x` does nothing, so a key pressed while picking a file
    /// can never change a todo.
    fn toggle(&mut self) -> Option<Action> {
        match self.pane {
            Pane::History => self.toggle_entry(),
            Pane::View | Pane::Todos => {
                let task = self.current_task()?;
                let inverse = self.doc.check(task.line)?;
                let after = self.task_at(task.line)?;
                let action = if after.done { "check" } else { "uncheck" };
                Some(self.record(label(action, &after), inverse))
            }
            Pane::Files | Pane::Outline => None,
        }
    }

    /// Uncheck a history entry to undo its write; check it again to redo it.
    fn toggle_entry(&mut self) -> Option<Action> {
        let global = *self.file_history().get(self.history_idx)?;
        let entry = self.history.get(global)?.clone();
        let inverse = if entry.applied {
            entry.forward.clone().flip()
        } else {
            entry.forward.clone()
        };
        match self.doc.apply(inverse) {
            Ok(()) => {
                self.history[global].applied = !entry.applied;
                Some(self.save_action())
            }
            Err(mismatch) => {
                self.set_message(format!("{mismatch} — press r to reload"));
                None
            }
        }
    }

    /// `d`: delete the selected todo, or forget the selected history entry.
    ///
    /// Deleting a todo asks first, because the line is gone afterwards. Forgetting
    /// a history entry does not: the write it undoes is being reverted at the same
    /// time, and the entry itself is what was being deleted, so there is nothing
    /// left to ask about and nothing to record.
    fn ask_delete(&mut self) -> Option<Action> {
        match self.pane {
            Pane::History => self.forget_entry(),
            Pane::View | Pane::Todos => {
                if self.current_task().is_some() {
                    self.mode = Mode::Confirming;
                }
                None
            }
            Pane::Files | Pane::Outline => None,
        }
    }

    /// Undo a history entry's write and drop the entry. A refused revert leaves
    /// both alone, because an applied write is not something to forget silently.
    fn forget_entry(&mut self) -> Option<Action> {
        let global = *self.file_history().get(self.history_idx)?;
        let entry = self.history.get(global)?.clone();
        if !entry.applied {
            // already undone: nothing to write, the entry just goes away
            self.history.remove(global);
            self.history_idx = self
                .history_idx
                .min(self.file_history().len().saturating_sub(1));
            return None;
        }
        if let Err(mismatch) = self.doc.apply(entry.forward.clone().flip()) {
            self.set_message(format!("{mismatch} — press r to reload"));
            return None;
        }
        self.history.remove(global);
        self.history_idx = self
            .history_idx
            .min(self.file_history().len().saturating_sub(1));
        Some(self.save_action())
    }

    /// `a`: an empty prompt for a new todo.
    fn ask_add(&mut self) -> Option<Action> {
        self.mode = Mode::Adding(Prompt::new(""));
        None
    }

    /// `e`: the selected todo's body, ready to edit.
    fn ask_edit(&mut self) -> Option<Action> {
        let task = self.current_task()?;
        self.mode = Mode::Editing(Prompt::new(&task.body));
        None
    }

    fn add(&mut self, body: &str) -> Option<Action> {
        let after = self.current_task().map(|task| task.line);
        let inverse = self.doc.add_after(after, body)?;
        let aricma_todo_lib::model::Inverse::Insert { line, .. } = inverse else {
            return None;
        };
        self.task_idx = self
            .tasks()
            .iter()
            .position(|task| task.line == line)
            .unwrap_or(0);
        let after = self.task_at(line)?;
        Some(self.record(label("add", &after), inverse))
    }

    fn edit(&mut self, body: &str) -> Option<Action> {
        let task = self.current_task()?;
        let inverse = self.doc.set_body(task.line, body)?;
        let after = self.task_at(task.line)?;
        Some(self.record(label("edit", &after), inverse))
    }

    fn priority(&mut self, prio: Option<Prio>) -> Option<Action> {
        let task = self.current_task()?;
        // the letter a todo already carries clears it: () -A-> (A) -A-> ()
        let prio = match prio {
            Some(prio) if Some(prio) == task.prio => None,
            other => other,
        };
        let inverse = self.doc.set_prio(task.line, prio)?;
        let after = self.task_at(task.line)?;
        let action = match prio {
            Some(prio) => format!("priority {}", prio.letter()),
            None => "priority clear".to_owned(),
        };
        Some(self.record(label(&action, &after), inverse))
    }

    /// One task by its line, read back after an edit.
    fn task_at(&self, line: usize) -> Option<Task> {
        self.tasks().into_iter().find(|task| task.line == line)
    }
}

/// A history row says what was done, then what it was done to: `check · pay
/// rent`. The whole line would be unreadable in the sidebar, and the state is
/// already the checkbox in front of it.
fn label(action: &str, task: &Task) -> String {
    let subject = if task.title.trim().is_empty() {
        &task.body
    } else {
        &task.title
    };
    format!("{action} · {subject}")
}
