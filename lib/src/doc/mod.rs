//! The document: a file held as exact lines, with tasks derived from them.
//!
//! Nothing in this module re-renders a line it does not own. `parse` reads,
//! `render` writes the same lines back, and the edits in `edit` touch exactly
//! one line each.

pub mod edit;
pub mod findings;
pub mod format;
pub mod headings;
pub mod parse;
pub mod render;
pub mod spans;

/// A markdown file as exact lines, with no `\n` and no trailing-newline entry.
#[derive(Clone)]
pub struct Doc {
    pub(crate) lines: Vec<String>,
}

impl Doc {
    /// The raw lines, for a view that renders the file as it is.
    pub fn lines(&self) -> &[String] {
        &self.lines
    }
}
