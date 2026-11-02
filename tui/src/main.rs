//! aricma-lazytodo — the screen. Everything it knows about the rules comes
//! from the lib; this crate is state, keys, drawing and the event loop.
#![forbid(unsafe_code)]

mod action;
mod app;
mod entry;
mod keys;
mod prompt;
#[cfg(test)]
mod testing;
mod ui;

pub use action::Action;
pub use app::App;

use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use aricma_todo_lib::error::Error;
use aricma_todo_lib::store;
use clap::Parser;

/// Read and edit the todo lines of a markdown file.
#[derive(Debug, Parser)]
#[command(name = "aricma-lazytodo", version, about)]
struct Args {
    /// A file or a directory to open. Defaults to the current directory.
    path: Option<PathBuf>,
}

fn main() -> ExitCode {
    match run(&Args::parse().path.unwrap_or_else(|| PathBuf::from("."))) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("aricma-lazytodo: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Open the screen and pump events until the user quits.
///
/// Refuses to start without a terminal: `ratatui::init` would panic, and a
/// broken TUI is a worse answer than "use the CLI".
fn run(path: &Path) -> Result<ExitCode, Error> {
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        return Err(Error::Ops(
            "the TUI needs a terminal; use aricma-todo for check, format, export or import"
                .to_owned(),
        ));
    }
    let mut app = App::open(path)?;
    let mut terminal = ratatui::init();
    let outcome = loop_app(&mut terminal, &mut app);
    ratatui::restore();
    outcome.map(|()| ExitCode::SUCCESS)
}

/// How long the loop waits for a key before looking at the file on disk.
///
/// The poll is what makes an external write visible without anyone asking: the
/// file is small, and one `stat` plus one read a second is cheaper than a person
/// wondering why the screen is stale.
const TICK: std::time::Duration = std::time::Duration::from_millis(750);

fn loop_app(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> Result<(), Error> {
    use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

    while !app.quit {
        terminal.draw(|frame| ui::draw(frame, app))?;
        if !event::poll(TICK)? {
            if let Err(error) = app.refresh_from_disk() {
                app.set_message(error.to_string());
            }
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        let control = key.modifiers.contains(KeyModifiers::CONTROL);
        let action = match key.code {
            KeyCode::Char('c') if control => Some(Action::Quit),
            // cmd+r cannot reach a terminal app; ctrl+r is the key that does
            KeyCode::Char('r') if control => Some(Action::Reload),
            _ => app.on_key(key.code),
        };
        if let Some(action) = action {
            perform(app, action);
        }
    }
    Ok(())
}

/// The only place an `Action` turns into disk or terminal state.
fn perform(app: &mut App, action: Action) {
    match action {
        Action::Save { text, .. } => match store::save(&app.baseline, &text) {
            Ok(saved) => app.saved(saved),
            Err(error) => app.set_message(error.to_string()),
        },
        Action::Reload => {
            if let Err(error) = app.reload() {
                app.set_message(error.to_string());
            }
        }
        Action::Quit => app.quit = true,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use ratatui::crossterm::event::KeyCode;

    use crate::{perform, App};

    #[test]
    fn a_save_that_hits_a_stale_file_keeps_the_edit_and_says_so() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("TODOs.md");
        fs::write(&path, "- [ ] a\n").unwrap();
        let mut app = App::open(&path).unwrap();
        app.on_key(KeyCode::Char('x')); // the single-file session starts on the index
        fs::write(&path, "- [ ] written by someone else\n").unwrap();

        // the edit is still in memory, and the message explains the refusal
        let action = app.save_action();
        perform(&mut app, action);
        assert_eq!(app.doc.render(), "- [x] a\n");
        let message = app.message.clone().unwrap_or_default();
        assert!(message.contains("changed on disk"), "{message}");

        let action = app.on_key(KeyCode::Char('q')).unwrap();
        perform(&mut app, action);
        assert!(app.quit);
    }
}
