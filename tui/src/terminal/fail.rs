//! Say why nothing started, and fail.

use std::process::ExitCode;

use aricma_todo_lib::error::Error;

pub fn fail(error: &Error) -> ExitCode {
    eprintln!("aricma-lazytodo: {error}");
    ExitCode::FAILURE
}
