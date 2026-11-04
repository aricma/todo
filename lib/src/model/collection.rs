//! A tag or a person, and the todos that carry it.
//!
//! A collection is not a file and not a folder: it is every todo in the tree that
//! mentions one `#tag` or one `@person`. The scan counts them; the collection view
//! gathers them, and each of them keeps the file it came from.

use std::fmt;
use std::path::PathBuf;

/// Which of the two kinds of collection a name belongs to.
///
/// They are separate namespaces: `#sam` is a tag and `@sam` is a person, and one
/// can exist without the other.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Tag,
    Assignee,
}

impl Kind {
    /// The sigil the name is written with.
    pub const fn sigil(self) -> char {
        match self {
            Self::Tag => '#',
            Self::Assignee => '@',
        }
    }

    /// How the interface names this kind: `tags`, `people`.
    pub const fn plural(self) -> &'static str {
        match self {
            Self::Tag => "tags",
            Self::Assignee => "people",
        }
    }
}

/// One collection: `#home`, with how many todos in the tree carry it and how many
/// of those are done.
///
/// `count` is a count of todos, not of files, and a todo that carries a tag twice
/// counts once — the collection is a set of todos. `done` is what makes the row
/// read like a file's: `2/5`, not `5`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Collection {
    pub kind: Kind,
    /// The name without its sigil, compared exactly: tags are case sensitive.
    pub name: String,
    pub count: usize,
    pub done: usize,
}

impl fmt::Display for Collection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}{}", self.kind.sigil(), self.name)
    }
}

/// One todo as a collection sees it: the same todo from its own file.
///
/// The line is the anchor — it is how a write finds the task again — and `done` is
/// read at gather time so a collection can say how much of it is finished without
/// opening every file again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    /// The file this todo lives in. Every write from a collection goes here.
    pub path: PathBuf,
    pub line: usize,
    /// The indent, the bullet and the checkbox, as written: what a view draws
    /// before the body so the line on screen is the line in the file.
    pub lead: String,
    /// The body as written.
    pub body: String,
    pub done: bool,
}
