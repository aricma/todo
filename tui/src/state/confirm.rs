//! What a `d` is about to delete, while it is being asked about.
//!
//! One question at a time, so the answer cannot be given to the wrong thing.

/// The two things `d` can remove.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Confirm {
    /// One todo line, in the file that is open.
    Todo,
    /// The whole todo file, from the disk.
    File,
}
