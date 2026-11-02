//! `import`: append the todos of an OPS document to a markdown file.

use std::path::Path;
use std::process::ExitCode;

use aricma_todo_lib::{doc::Doc, error::Error, ops, store};

use crate::args::Import;
use crate::exit;

pub fn run(import: &Import) -> Result<ExitCode, Error> {
    let target = import
        .into
        .clone()
        .unwrap_or_else(|| next_to_todos(&import.file));
    let bytes = store::load(&import.file)?;
    let format = ops::format_from(import.r#as.as_deref().unwrap_or_default(), &import.file);
    let loaded = store::load(&target)?;
    let mut doc = Doc::parse(&loaded.text);
    let added = ops::append(&mut doc, &bytes.text, format)?;
    if added > 0 {
        store::save(&loaded, &doc.render())?;
    }
    println!("{added} added to {}", target.display());
    Ok(ExitCode::from(exit::OK))
}

/// Where tasks go when `--into` is absent: `TODOs.md` beside the document.
fn next_to_todos(file: &Path) -> std::path::PathBuf {
    file.parent()
        .unwrap_or_else(|| Path::new("."))
        .join("TODOs.md")
}
