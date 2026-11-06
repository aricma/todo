use std::path::PathBuf;

use crate::doc::Source;
use crate::model::{Collection, Stats};

/// One candidate file: where it is, how its tasks count, and whether it is too
/// long to open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub path: PathBuf,
    /// What its todos are: checkboxes in a todo document, or todo comments in a code
    /// file — `TODO:`, `FIXME:`, `HACK:`, `XXX:` (RFC-015, RFC-020).
    pub source: Source,
    pub stats: Stats,
    /// More lines than [`crate::limits::MAX_LINES`]: shown, marked, not read.
    pub too_long: bool,
    /// The tags and people this file's todos carry, with how many todos each — what
    /// the collection rows are summed from.
    pub collections: Vec<Collection>,
}

impl Entry {
    /// Why this file is not read, when it is not.
    pub fn skipped_reason(&self) -> Option<String> {
        self.too_long
            .then(|| format!("more than {} lines, not read", crate::limits::MAX_LINES))
    }
}
