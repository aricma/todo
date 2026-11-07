//! Test helpers shared by the integration suites: run a binary, read what it
//! said, lay down a fixture, drive a use case folder.
//!
//! A plain dev-dependency of the test targets, so no suite owns the helpers and
//! a third suite would not need to copy them either.
#![allow(dead_code)] // each suite uses the part it needs

pub mod cases;

pub use cases::{Case, CaseResult, Cli, Tui, driver_lines, eventually, expect, expect_contains};

use std::fs;
use std::path::Path;
use std::process::{Command, Output, Stdio};

/// Run a binary, with stdin closed so nothing that reads it can wait for a key.
pub fn run(binary: &str, arguments: &[&str]) -> Output {
    Command::new(binary)
        .args(arguments)
        .stdin(Stdio::null())
        .output()
        .expect("the binary runs")
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Write a file, creating its directory; answer with its path as a string.
pub fn file(root: &Path, name: &str, text: &str) -> String {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, text).unwrap();
    path.to_string_lossy().into_owned()
}
