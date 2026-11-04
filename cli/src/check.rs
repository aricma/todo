//! `--check`: report the todo rules and exit non-zero if any is broken.

use std::path::Path;
use std::process::ExitCode;

use aricma_todo_lib::{discover, doc::Doc, error::Error, store};

use crate::exit;

/// What the limits stopped, on stderr: `--check` must not imply it saw more than
/// it did.
pub(crate) fn report(scan: &discover::Scan) {
    for warning in &scan.warnings {
        eprintln!("aricma-todo: {warning}");
    }
}

pub fn run(path: &Path) -> Result<ExitCode, Error> {
    let mut found = false;
    let scan = discover::scan(path)?;
    report(&scan);
    for file in scan.entries {
        if let Some(reason) = file.skipped_reason() {
            eprintln!("{}: {reason}", file.path.display());
            continue;
        }
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
