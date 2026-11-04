//! Make the folders and an empty file, and never overwrite one.

use std::io::Write;
use std::path::Path;

use aricma_todo_lib::error::Error;

/// `create_new`, not `write`: an existing file comes back as `AlreadyExists`, so
/// nothing is overwritten even if one appears between the field's check and this
/// call. The text is the caller's — a new file is not always empty.
pub fn create(file: &Path, text: &str) -> Result<(), Error> {
    if let Some(folder) = file.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let mut made = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(file)?;
    made.write_all(text.as_bytes())?;
    Ok(())
}
