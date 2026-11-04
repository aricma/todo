use std::path::PathBuf;

/// One todo the search found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    pub path: PathBuf,
    /// The line the todo is on, so jumping to it needs no second lookup.
    pub line: usize,
    /// The body as written: what the result row shows.
    pub body: String,
    /// Higher is a better match; ties keep file order.
    pub score: u32,
}
