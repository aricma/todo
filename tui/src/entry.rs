//! One session-history row: a write that can be unchecked.

use std::path::PathBuf;

use aricma_todo_lib::model::Inverse;

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
}
