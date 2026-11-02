//! What the binary does, as a script sees it: stdout, stderr and the exit code.

use std::fs;

use testkit::{file, run, stderr, stdout};

const BIN: &str = env!("CARGO_BIN_EXE_aricma-todo");

#[test]
fn check_reports_a_rule_violation_and_exits_one() {
    let tmp = tempfile::tempdir().unwrap();
    let path = file(tmp.path(), "TODOs.md", "# t\n- [ ]no space\n");
    let output = run(BIN, &["check", &path]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stdout(&output).contains(":2: malformed task line"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn check_is_quiet_and_zero_on_a_valid_file() {
    let tmp = tempfile::tempdir().unwrap();
    let path = file(tmp.path(), "TODOs.md", "- [X] (B) a #x @y   \n- [ ] b\n");
    let output = run(BIN, &["check", &path]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout(&output), "");
}

#[test]
fn check_flags_a_bad_priority_and_an_empty_title() {
    let tmp = tempfile::tempdir().unwrap();
    let path = file(
        tmp.path(),
        "TODOs.md",
        "- [ ] (E) x\n- [ ] (A) #only @tokens\n",
    );
    let output = run(BIN, &["check", &path]);
    assert_eq!(output.status.code(), Some(1));
    let text = stdout(&output);
    assert!(text.contains("priority must be A-D"), "{text}");
    assert!(text.contains("no title"), "{text}");
}

#[test]
fn check_walks_a_directory() {
    let tmp = tempfile::tempdir().unwrap();
    file(tmp.path(), "TODOs.md", "- [ ] fine\n");
    file(tmp.path(), "deep/TASKS.md", "- [ ]no space\n");
    file(tmp.path(), "notes.md", "- [ ]no space\n");
    let output = run(BIN, &["check", tmp.path().to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output).lines().count(), 1, "{}", stdout(&output));
}

#[test]
fn format_fixes_the_file_and_check_then_passes() {
    let tmp = tempfile::tempdir().unwrap();
    let path = file(tmp.path(), "TODOs.md", "- [X] (b) a  #x  \n");
    assert_eq!(run(BIN, &["format", &path]).status.code(), Some(0));
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "- [x] (B) a  #x  \n",
        "only the checkbox and the letter move"
    );
    assert_eq!(run(BIN, &["check", &path]).status.code(), Some(0));
}

#[test]
fn format_leaves_a_clean_file_alone() {
    let tmp = tempfile::tempdir().unwrap();
    let source = "# t\n\n- [x] (A) a\n";
    let path = file(tmp.path(), "TODOs.md", source);
    let before = fs::metadata(&path).unwrap().modified().unwrap();
    assert_eq!(run(BIN, &["format", &path]).status.code(), Some(0));
    assert_eq!(fs::read_to_string(&path).unwrap(), source);
    assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), before);
}

#[test]
fn help_exits_zero_and_lists_export_and_import() {
    let output = run(BIN, &["--help"]);
    assert_eq!(output.status.code(), Some(0));
    let text = stdout(&output);
    for verb in ["check", "format", "export", "import"] {
        assert!(text.contains(verb), "{verb} missing from help:\n{text}");
    }
}

#[test]
fn no_subcommand_is_a_usage_error() {
    let output = run(BIN, &[]);
    assert_eq!(output.status.code(), Some(2));
    let text = format!("{}{}", stdout(&output), stderr(&output));
    for verb in ["check", "format", "export", "import"] {
        assert!(text.contains(verb), "{verb} missing from usage:\n{text}");
    }
}

#[test]
fn a_missing_path_exits_two_with_a_message() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("nope.md");
    let output = run(BIN, &["check", path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("no such file or directory"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn export_writes_ops_json_and_import_brings_the_line_back() {
    let tmp = tempfile::tempdir().unwrap();
    let path = file(tmp.path(), "TODOs.md", "- [ ] (A) @dave fix you #bug\n");
    let exported = run(BIN, &["export", &path, "--as", "json"]);
    assert_eq!(exported.status.code(), Some(0), "{}", stderr(&exported));
    let json = stdout(&exported);
    assert!(json.contains("\"@dave fix you #bug\""), "{json}");
    assert!(json.contains("\"priority\": \"A\""), "{json}");

    let target = file(tmp.path(), "TASKS.md", "");
    let document = tmp.path().join("export.json");
    fs::write(&document, &json).unwrap();
    let imported = run(
        BIN,
        &[
            "import",
            document.to_str().unwrap(),
            "--into",
            &target,
            "--as",
            "json",
        ],
    );
    assert_eq!(imported.status.code(), Some(0), "{}", stderr(&imported));
    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        "- [ ] (A) @dave fix you #bug\n"
    );
}
