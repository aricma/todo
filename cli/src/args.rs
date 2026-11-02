//! The command line: what to point at, and what to do with it.

use std::path::PathBuf;

use clap::{Args as ClapArgs, Parser, Subcommand};

/// Check, format, export and import todo lines in markdown files.
#[derive(Debug, Parser)]
#[command(name = "aricma-todo", version, about)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Report todo-rule violations and exit 1. Never writes.
    Check(Target),
    /// Fix the checkbox case and the priority letter case.
    Format(Target),
    /// Write every todo as one OPS document.
    Export(Export),
    /// Append the todos of an OPS document.
    Import(Import),
}

/// A file or a directory, defaulting to the current directory.
#[derive(Debug, ClapArgs)]
pub struct Target {
    pub path: Option<PathBuf>,
}

#[derive(Debug, ClapArgs)]
pub struct Export {
    /// The file or directory to read. Defaults to the current directory.
    pub from: Option<PathBuf>,

    /// OPS format: json, yaml, jsonl or csv.
    #[arg(long, value_name = "FORMAT", default_value = "json")]
    pub r#as: String,

    /// Write here instead of stdout.
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, ClapArgs)]
pub struct Import {
    /// The OPS document to read.
    pub file: PathBuf,

    /// The markdown file to append to. Defaults to `TODOs.md` next to the
    /// document.
    #[arg(long)]
    pub into: Option<PathBuf>,

    /// OPS format: json, yaml, jsonl or csv. Inferred from the file when absent.
    #[arg(long, value_name = "FORMAT")]
    pub r#as: Option<String>,
}
