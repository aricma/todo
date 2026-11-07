//! What the binary was asked to open.

use std::path::PathBuf;

use clap::Parser;

/// Read and edit the todo lines of a markdown file.
#[derive(Debug, Parser)]
#[command(name = "lazytodo", version, about)]
struct Args {
    /// A file or a directory to open. Defaults to the current directory.
    path: Option<PathBuf>,
}

/// The path to open, or the current directory.
pub fn path() -> PathBuf {
    Args::parse().path.unwrap_or_else(|| PathBuf::from("."))
}
