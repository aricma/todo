//! `--format`: the two mechanical fixes, through the same atomic save the TUI
//! uses.

use std::path::Path;
use std::process::ExitCode;

use aricma_todo_lib::{discover, doc::Doc, error::Error, store};

use crate::exit;

pub fn run(path: &Path) -> Result<ExitCode, Error> {
    let scan = discover::scan(path)?;
    // a limit that stopped the walk must be said out loud here too: formatting
    // fewer files than exist is not something to discover later
    crate::check::report(&scan);
    for file in scan.entries {
        if file.too_long {
            eprintln!(
                "{}: more than {} lines, not formatted",
                file.path.display(),
                aricma_todo_lib::limits::MAX_LINES
            );
            continue;
        }
        let loaded = store::load(&file.path)?;
        let mut doc = Doc::parse(&loaded.text);
        if doc.format() {
            store::save(&loaded, &doc.render())?;
        }
    }
    Ok(ExitCode::from(exit::OK))
}
