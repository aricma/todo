use std::path::PathBuf;

use crate::model::Stats;

/// One candidate file: where it is, and how its tasks count.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub path: PathBuf,
    pub stats: Stats,
}
