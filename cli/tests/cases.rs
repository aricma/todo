//! Every `use-cases/cli/**/case.txt` is one test. See `use-cases/README.md`.

use std::fs;
use std::path::Path;
use std::process::Command;

use testkit::cases::{driver_lines, expect, read_optional, Case, CaseResult};

// `harness!` generates `main`, so it belongs at module level.
datatest_stable::harness! {
    { test = run, root = concat!(env!("CARGO_MANIFEST_DIR"), "/../use-cases/cli"), pattern = r"^.*case\.txt$" },
}

/// One case folder, driven through the CLI binary.
fn run(case_file: &Path) -> CaseResult {
    let folder = case_file.parent().ok_or("case.txt without a folder")?;
    let case = Case::open(case_file)?;
    let argv = driver_lines(&fs::read_to_string(folder.join("command.txt"))?);
    if argv.is_empty() {
        return Err(format!("{}: no command line", folder.display()).into());
    }
    let arguments: Vec<String> = argv
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
    let expectations = &case.expectations;
    expect(
        &output.status.code(),
        &Some(expectations.exit),
        &format!("exit code (stderr: {stderr})"),
    )?;
    expectations.check_stderr(&stderr)?;
    let exact = read_optional(&folder.join("stdout.md"))?;
    expectations.check_stdout(&stdout, exact.as_deref())?;
    case.check_file()
}
