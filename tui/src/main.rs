//! lazytodo — the screen.
//!
//! Everything it knows about the rules comes from the lib, which also reads and
//! writes a todo file. The layers around it: `disk` holds the file operations the
//! lib has no opinion on (create, delete, measure, list folders), `state` is the
//! machine the screen is a function of, `ui` draws that state, `terminal` is the
//! loop that joins them, and `text` is every word the user reads.
#![forbid(unsafe_code)]

mod disk;
mod helpers;
mod state;
mod terminal;
#[cfg(test)]
mod testing;
mod text;
mod ui;

use std::process::ExitCode;

use state::App;

fn main() -> ExitCode {
    // the path first: `--help` and `--version` are answers a script asks for with
    // no terminal anywhere in sight
    let path = terminal::args::path();
    if let Err(error) = terminal::setup() {
        return terminal::fail(&error);
    }
    let mut app = match App::open(&path) {
        Ok(app) => app,
        Err(error) => return terminal::fail(&error),
    };
    terminal::run(&mut app)
}
