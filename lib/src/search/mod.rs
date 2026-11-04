//! Searching todos across every file the scan found.

pub mod find;
pub mod fuzzy;
pub mod hit;

pub use find::find;
pub use hit::Hit;
