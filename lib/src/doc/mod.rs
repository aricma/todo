//! The document: a file held as exact lines, with tasks derived from them.
//!
//! Nothing in this module re-renders a line it does not own. `parse` reads,
//! `render` writes the same lines back, and the edits in `edit` touch exactly
//! one line each.

pub mod comment;
pub mod edit;
pub mod findings;
pub mod format;
pub mod headings;
pub mod parse;
pub mod render;
pub mod spans;

pub use comment::Syntax;

/// Where a file's todos come from.
///
/// Three things in the tool hold todos — a todo document, a comment todo file, and a
/// collection gathered from the first two — and each allows a different set of
/// keys, because not every edit is possible in every one of them (RFC-015).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// A todo document: markdown whose `- [ ]` lines are the todos.
    Document,
    /// A comment todo file: anything else in the tree whose todo comments are the
    /// todos, one line each, with the code around them left alone.
    Comment,
}

impl Source {
    /// What a path is: markdown is a todo document, anything else is read for its
    /// todo comments. Markdown is never read for comments — a `<!-- TODO: -->` in
    /// a document belongs to the prose, and this tool does not hunt for it (RFC-016).
    pub fn of(path: &std::path::Path) -> Self {
        if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
        {
            Self::Document
        } else {
            Self::Comment
        }
    }
}

/// A file as exact lines, with no `\n` and no trailing-newline entry.
#[derive(Clone)]
pub struct Doc {
    pub(crate) lines: Vec<String>,
    pub(crate) source: Source,
    /// Which markers this file's language starts a comment with. Only a comment todo
    /// file reads it, and it is `Syntax::Any` for a document and for a file whose name
    /// a caller did not have (RFC-020).
    pub(crate) syntax: Syntax,
}

impl Doc {
    /// The raw lines, for a view that renders the file as it is.
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// Which kind of file this is: what its todos are, and what may be done to them.
    pub fn source(&self) -> Source {
        self.source
    }

    /// Parse a file as what its path says it is. Every reader uses this, so a file
    /// read by the search, by a collection and by the editor is the same file — and
    /// the comment markers come from the same name the source does, which is why this
    /// takes a path rather than a `Source` (RFC-020).
    pub fn parse_at(path: &std::path::Path, text: &str) -> Self {
        match Source::of(path) {
            Source::Document => Self::parse(text),
            Source::Comment => Self::parse_comment_in(Syntax::of(path), text),
        }
    }

    /// A document with no lines of its own, parsed as a comment todo file — for tests and
    /// for the empty state of a pane.
    pub fn empty(source: Source) -> Self {
        Self {
            lines: Vec::new(),
            source,
            syntax: Syntax::Any,
        }
    }
}
