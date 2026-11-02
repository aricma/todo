//! `--format`: the two mechanical fixes, through the same atomic save the TUI
//! uses.

use std::path::Path;
use std::process::ExitCode;

use aricma_todo_lib::{discover, doc::Doc, error::Error, store};

use crate::exit;

pub fn run(path: &Path) -> Result<ExitCode, Error> {
    for file in discover::scan(path)? {
        let loaded = store::load(&file.path)?;
        let mut doc = Doc::parse(&loaded.text);
        if doc.format() {
            store::save(&loaded, &doc.render())?;
        }
    }
    Ok(ExitCode::from(exit::OK))
}
