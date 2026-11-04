//! A row of the file panel: a file on disk, or a collection of todos.

use aricma_todo_lib::model::{Collection, Kind};

/// Which collections the file panel lists.
///
/// A repository with forty tags would double the panel, so what is listed is a
/// choice, and `c` cycles it: both, tags only, people only, files only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Showing {
    Both,
    Tags,
    People,
    Files,
}

impl Showing {
    /// Whether a collection of this kind is listed.
    pub fn wants(self, kind: Kind) -> bool {
        match self {
            Self::Both => true,
            Self::Tags => kind == Kind::Tag,
            Self::People => kind == Kind::Assignee,
            Self::Files => false,
        }
    }

    /// The next state in the cycle.
    pub fn next(self) -> Self {
        match self {
            Self::Both => Self::Tags,
            Self::Tags => Self::People,
            Self::People => Self::Files,
            Self::Files => Self::Both,
        }
    }
}

/// The two things the file panel lists. Files come first, then collections, so the
/// rows only move when a file or a tag appears — not when a todo is ticked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Row {
    /// An index into `App::files`.
    File(usize),
    /// Every todo in the tree that carries this tag or person.
    Collection(Collection),
}

impl Row {
    /// The file this row stands for, when it is one.
    pub fn file(&self) -> Option<usize> {
        match self {
            Self::File(index) => Some(*index),
            Self::Collection(_) => None,
        }
    }

    /// The collection this row stands for, when it is one.
    pub fn collection(&self) -> Option<&Collection> {
        match self {
            Self::Collection(collection) => Some(collection),
            Self::File(_) => None,
        }
    }
}
