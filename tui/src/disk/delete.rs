//! Remove a file.

use std::path::Path;

use aricma_todo_lib::error::Error;

pub fn delete(path: &Path) -> Result<(), Error> {
    std::fs::remove_file(path).map_err(Error::from)
}
