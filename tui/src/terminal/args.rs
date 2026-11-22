//! What the binary was asked to open.

use std::path::PathBuf;

use clap::Parser;

use crate::text;

/// Read and edit the todo lines of a markdown file.
#[derive(Debug, Parser)]
// the same number the status bar shows, v and all: two spellings of one version
// is one of them being wrong
#[command(
    name = "lazytodo",
    version = text::VERSION,
    about,
    // `--version` is one number asked for by name; `--help` is read by someone who
    // does not know yet which number they have, so it opens with it too
    help_template = "{before-help}{name} {version}\n{about-with-newline}\n{usage-heading} {usage}\n\n{all-args}{after-help}"
)]
struct Args {
    /// A file or a directory to open. Defaults to the current directory.
    path: Option<PathBuf>,
}

/// The path to open, or the current directory.
pub fn path() -> PathBuf {
    Args::parse().path.unwrap_or_else(|| PathBuf::from("."))
}
