//! Draw, read, update: the loop, and nothing else in it.

use aricma_todo_lib::error::Error;

use super::consts::TICK;
use super::perform::perform;
use crate::state::App;
use crate::ui;

pub fn pump(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> Result<(), Error> {
    use ratatui::crossterm::event::{self, Event, KeyEventKind};

    while !app.quit {
        terminal.draw(|frame| ui::draw(frame, app))?;
        if !event::poll(TICK)? {
            if let Err(error) = app.refresh_from_disk() {
                app.set_error(error.to_string());
            }
            continue;
        }
        // A paste is not a hundred key presses: bracketed paste is on, so the
        // terminal hands over what was copied as one string, and `state` decides what
        // it means — text in a field, a todo anywhere todos live.
        let action = match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => app.on_key(key),
            Event::Paste(text) => app.on_paste(&text),
            _ => continue,
        };
        // one place decides what a key means, and the whole map is in `state`:
        // this loop only draws, reads, and performs what came back
        if let Some(action) = action {
            perform(app, action);
        }
    }
    Ok(())
}
