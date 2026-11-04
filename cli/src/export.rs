//! `export`: every todo, as one OPS document, on stdout or in a file.

use std::fs;
use std::path::Path;
use std::process::ExitCode;

use aricma_todo_lib::{discover, error::Error, ops};

use crate::args::Export;
use crate::exit;

pub fn run(path: &Path, export: Export) -> Result<ExitCode, Error> {
    let files = discover::scan(path)?.entries;
    let format = ops::format_from(&export.r#as, &export.out.clone().unwrap_or_default());
    let exported = ops::write(&files, format)?;
    for skipped in &exported.skipped {
        eprintln!("{skipped}, skipped");
    }
    match export.out {
        Some(out) => fs::write(out, exported.text)?,
        None => print!("{}", exported.text),
    }
    Ok(ExitCode::from(exit::OK))
}
