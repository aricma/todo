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

pub use export::write;
pub use exported::Exported;
pub use format::format_from;
pub use import::append;
