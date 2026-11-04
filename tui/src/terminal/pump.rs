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
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        // one place decides what a key means, and the whole map is in `state`:
        // this loop only draws, reads, and performs what came back
        if let Some(action) = app.on_key(key) {
            perform(app, action);
        }
    }
    Ok(())
}
