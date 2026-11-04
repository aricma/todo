//! The only place an `Action` turns into disk or terminal state.

use crate::state::{Action, App};
use std::io::Write;

use aricma_todo_lib::error::Error;
use aricma_todo_lib::store;

use crate::helpers::base64;

pub fn perform(app: &mut App, action: Action) {
    match action {
        Action::Save { text } => match store::save(&app.baseline, &text) {
            Ok(saved) => app.saved(saved),
            // the file moved under the write: the edit is still in memory, and only
            // the person in front of it can say which version to keep
            Err(error @ Error::StaleFile(_)) => {
                let _ = error;
                app.conflict();
            }
            Err(error) => app.set_error(error.to_string()),
        },
        Action::Overwrite { text } => match store::overwrite(&app.baseline, &text) {
            Ok(saved) => app.saved(saved),
            Err(error) => app.set_error(error.to_string()),
        },
        Action::Copy { text } => copy(&text),
        Action::Quit => app.quit = true,
    }
}

/// Put text on the terminal's clipboard: OSC 52, which is the only clipboard a
/// program on the other side of a pty can reach.
///
/// A terminal that does not implement it ignores the sequence, and there is no way
/// to find out: the write goes out either way, and the message says what was copied
/// rather than promising it landed.
fn copy(text: &str) {
    let sequence = format!("\u{1b}]52;c;{}\u{1b}\\", base64(text.as_bytes()));
    let mut out = std::io::stdout();
    let _ = out.write_all(sequence.as_bytes());
    let _ = out.flush();
}
