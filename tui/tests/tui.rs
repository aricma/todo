//! What the TUI binary does, as a script sees it: it refuses to run without a
//! terminal, and it says which binary to use instead.

use std::fs;

use testkit::{run, stderr};

const BIN: &str = env!("CARGO_BIN_EXE_lazytodo");

#[test]
fn the_tui_refuses_to_start_without_a_terminal() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("TODOs.md"), "- [ ] a\n").unwrap();
    let output = run(BIN, &[tmp.path().to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("needs a terminal"),
        "{}",
        stderr(&output)
    );
    assert!(
        stderr(&output).contains("use aricma-todo "),
        "it names the CLI binary: {}",
        stderr(&output)
    );
}

#[test]
fn help_exits_zero() {
    let output = run(BIN, &["--help"]);
    assert_eq!(output.status.code(), Some(0));
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(text.contains("lazytodo"), "{text}");
}
