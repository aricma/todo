//! Every `cli.txt` under `use-cases/` is one test, driven through the CLI binary.
//! See `use-cases/README.md`.
//!
//! `cli.txt` is the entry point on purpose: a core use case the CLI cannot do yet
//! simply has no driver file, so it does not become a test that could only be
//! skipped.

use std::path::Path;
use std::process::Command;

use testkit::cases::{expect, Case, CaseResult, Cli};

// `harness!` generates `main`, so it belongs at module level.
// one root, because a case may live in `core/` or in `cli/`: the pattern is what
// decides which interface it runs on, so a folder with no `cli.txt` is simply not
// collected
datatest_stable::harness! {
    { test = run, root = concat!(env!("CARGO_MANIFEST_DIR"), "/../use-cases"), pattern = r"cli\.txt$" },
}

/// One case folder, driven through the CLI binary.
fn run(driver: &Path) -> CaseResult {
    let case = Case::open(driver)?;
    let cli = Cli::read(driver)?;
    let arguments: Vec<String> = cli
        .argv
        .iter()
        .flat_map(|line| line.split_whitespace())
        .map(|argument| case.substitute(argument))
        .collect();
    let output = Command::new(env!("CARGO_BIN_EXE_aricma-todo"))
        .args(&arguments)
        .current_dir(case.scratch.path())
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    expect(
        &output.status.code(),
        &Some(cli.exit),
        &format!("exit code (stderr: {stderr})"),
    )?;
    cli.check_stderr(&stderr)?;
    cli.check_stdout(&stdout, case.stdout.as_deref())?;
    case.check_file()
}
