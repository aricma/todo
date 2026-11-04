//! A screen, a loop, and a terminal left exactly as it was found.

use std::process::ExitCode;

use super::enhanced;
use super::fail::fail;
use super::pump::pump;
use crate::state::App;

pub fn run(app: &mut App) -> ExitCode {
    let mut terminal = ratatui::init();
    let outcome = pump(&mut terminal, app);
    enhanced::pop();
    ratatui::restore();
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => fail(&error),
    }
}
