//! aricma-todo — the pipeline interface. No screen, no ratatui.
#![forbid(unsafe_code)]

mod args;
mod check;
mod exit;
mod export;
mod format;
mod import;

use std::path::PathBuf;
use std::process::ExitCode;

use aricma_todo_lib::error::Error;
use clap::Parser;

fn main() -> ExitCode {
    match run(args::Args::parse()) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("aricma-todo: {error}");
            ExitCode::from(exit::USAGE)
        }
    }
}

fn run(arguments: args::Args) -> Result<ExitCode, Error> {
    match arguments.command {
        args::Command::Check(target) => check::run(&or_current(target.path)),
        args::Command::Format(target) => format::run(&or_current(target.path)),
        args::Command::Export(export) => export::run(&or_current(export.from.clone()), export),
        args::Command::Import(import) => import::run(&import),
    }
}

fn or_current(path: Option<PathBuf>) -> PathBuf {
    path.unwrap_or_else(|| PathBuf::from("."))
}
