//! Everything that has to be true before a screen exists.

use aricma_todo_lib::error::Error;

use super::{enhanced, hook};

/// A terminal to draw on, and a way out of it that survives a panic.
///
/// Refuses to start without a terminal: `ratatui::init` would panic, and a broken
/// TUI is a worse answer than "use the CLI".
pub fn setup() -> Result<(), Error> {
    use std::io::IsTerminal;

    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        return Err(Error::Ops(
            "the TUI needs a terminal; use aricma-todo for check, format, export or import"
                .to_owned(),
        ));
    }
    hook::install();
    enhanced::push();
    Ok(())
}
