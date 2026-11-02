//! `--check`: report the todo rules and exit non-zero if any is broken.

use std::path::Path;
use std::process::ExitCode;

use aricma_todo_lib::{discover, doc::Doc, error::Error, store};

use crate::exit;

pub fn run(path: &Path) -> Result<ExitCode, Error> {
    let mut found = false;
    for file in discover::scan(path)? {
        let loaded = store::load(&file.path)?;
        let doc = Doc::parse(&loaded.text);
        for finding in doc.findings() {
            println!(
                "{}:{}: {}",
                file.path.display(),
                finding.line + 1,
                finding.message
            );
            found = true;
        }
    }
    Ok(ExitCode::from(if found {
        exit::FINDINGS
    } else {
        exit::OK
    }))
}
