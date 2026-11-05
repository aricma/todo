//! The Open Productivity Standard seam: our todo lines into OPS tasks and
//! back, through the reference `ops-lib`.
//!
//! The line stays the source of truth. Export removes the positional priority
//! token (it carries nothing a title needs) and puts it in `metadata.priority`,
//! while tags and assignees stay inside the title because they are how the line
//! reads. Import does the opposite and never re-derives tokens from metadata.

pub mod export;
pub mod exported;
pub mod format;
pub mod import;

pub use export::{one, write};
pub use exported::Exported;
pub use format::format_from;
pub use import::append;

use crate::doc::Doc;
use crate::error::Error;

/// Insert an OPS **JSON** document at `at`, in todo lines, and say which lines were
/// written: what a paste becomes.
///
/// JSON because that is what `c` puts on the clipboard for a todo, and because a
/// document copied out of another tool, an export or an agent arrives as JSON. The
/// file only ever grows todo lines — the document itself is not pasted (RFC-014).
pub fn paste(doc: &mut Doc, bytes: &str, at: usize) -> Result<Vec<usize>, Error> {
    import::insert_document(doc, bytes, ops_lib::v0::Format::Json, at)
}
