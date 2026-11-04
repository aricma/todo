//! Collections: every todo in the tree that carries one tag or one person.
//!
//! A folder is *where* a todo lives; a collection is *what it is about*, and the
//! two cut across each other. So a collection is never stored: it is derived from
//! the scan (which files mention it, and how often) and gathered from those files
//! when it is opened — the files on disk stay the only truth.

mod gather;
mod list;

pub use crate::model::Item;
pub use gather::gather;
pub use list::list;
