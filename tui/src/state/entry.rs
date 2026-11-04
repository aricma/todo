//! One session-history row: a write that can be undone.

use std::path::PathBuf;

use aricma_todo_lib::doc::Doc;
use aricma_todo_lib::model::{Inverse, Prio, Task};

use super::message::Message;

/// A write the TUI performed, and the edit that made it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The file it happened in; history is shown per file.
    pub path: PathBuf,
    /// What it did, as one line: `+ (A) buy milk`, `~ (B) ship #x`, `- [x] pay`.
    pub label: String,
    /// The edit as applied. `flip()` undoes it, and replaying it redoes it.
    pub forward: Inverse,
    /// Whether the edit is currently in the file.
    pub applied: bool,
    /// What this row has to say for itself — "that line changed since then" — or
    /// `None`. The row owns it: an index into the history would go stale the
    /// moment another row was forgotten.
    pub message: Option<Message>,
}

impl Entry {
    /// The line this write touched. A move touched two, and it is filed under the
    /// first of them.
    pub fn line(&self) -> usize {
        match self.forward {
            Inverse::Set { line, .. }
            | Inverse::Insert { line, .. }
            | Inverse::Remove { line, .. } => line,
            Inverse::Swap { a, .. } => a,
        }
    }

    /// Which part of the line it changed, and what that part read on each side.
    /// `None` for a write that added or removed a line, which has no field.
    pub fn change(&self) -> Option<Change> {
        let Inverse::Set { expect, text, .. } = &self.forward else {
            return None;
        };
        change_of(&task_of(expect)?, &task_of(text)?)
    }
}

/// The one part of a todo a write changed: the checkbox, the priority token, or
/// the body. Every mutation in the library owns exactly one of them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    Done {
        from: bool,
        to: bool,
    },
    Prio {
        from: Option<Prio>,
        to: Option<Prio>,
    },
    Body {
        from: String,
        to: String,
    },
}

impl Change {
    /// Whether this write put the line into the state `older` had just replaced:
    /// that is what makes `older` the write this one undoes.
    pub fn undoes(&self, older: &Self) -> bool {
        match (self, older) {
            (Self::Done { from, .. }, Self::Done { to: older_to, .. }) => from == older_to,
            (Self::Prio { from, .. }, Self::Prio { to: older_to, .. }) => from == older_to,
            (Self::Body { from, .. }, Self::Body { to: older_to, .. }) => from == older_to,
            _ => false,
        }
    }
}

/// Which field changed between two readings of one line, if exactly one did.
pub fn change_of(before: &Task, after: &Task) -> Option<Change> {
    if before.done != after.done {
        Some(Change::Done {
            from: before.done,
            to: after.done,
        })
    } else if before.prio != after.prio {
        Some(Change::Prio {
            from: before.prio,
            to: after.prio,
        })
    } else if before.body != after.body {
        Some(Change::Body {
            from: before.body.clone(),
            to: after.body.clone(),
        })
    } else {
        None
    }
}

/// The todo on one line, when that line is a todo at all.
pub fn task_of(line: &str) -> Option<Task> {
    Doc::parse(line).tasks().into_iter().next()
}
