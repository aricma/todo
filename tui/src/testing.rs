//! Helpers the TUI's own tests share, compiled only for tests.
//!
//! A test says what it wants on disk and what it expects on screen; the fixture
//! directory, the render and the key plumbing live here once.

use std::fs;

use ratatui::backend::TestBackend;
use ratatui::crossterm::event::KeyCode;
use ratatui::Terminal;

use crate::action::Action;
use crate::app::App;
use crate::ui;

/// The screen the tests render at, unless a test asks for another size.
pub const WIDTH: u16 = 80;
pub const HEIGHT: u16 = 24;

/// An app opened on a scratch directory holding `sources` as files.
///
/// The directory is returned with it: dropping it would delete the files the
/// app is still reading.
pub fn app(sources: &[(&str, &str)]) -> (tempfile::TempDir, App) {
    let scratch = tempfile::tempdir().unwrap();
    for (name, text) in sources {
        let path = scratch.path().join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    let app = App::open(scratch.path()).unwrap();
    (scratch, app)
}

/// An app opened on a single file rather than a folder: no file list, no file
/// pane.
pub fn single(source: &str) -> (tempfile::TempDir, App) {
    let scratch = tempfile::tempdir().unwrap();
    let path = scratch.path().join("TODOs.md");
    fs::write(&path, source).unwrap();
    let app = App::open(&path).unwrap();
    (scratch, app)
}

/// One key press.
pub fn press(app: &mut App, code: KeyCode) -> Option<Action> {
    app.on_key(code)
}

/// Type text one character at a time, as crossterm would deliver it.
pub fn type_text(app: &mut App, text: &str) {
    for character in text.chars() {
        app.on_key(KeyCode::Char(character));
    }
}

/// The text a `Save` action wants written, or a panic naming what came instead.
pub fn save_text(action: Option<Action>) -> String {
    match action {
        Some(Action::Save { text, .. }) => text,
        other => panic!("expected a save, got {other:?}"),
    }
}

/// An app rendered at a size, for a test that inspects styles as well as text.
pub fn terminal(app: &App, width: u16, height: u16) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| ui::draw(frame, app)).unwrap();
    terminal
}

/// The rendered screen as text, one string per row.
pub fn screen_at(app: &App, width: u16, height: u16) -> Vec<String> {
    let terminal = terminal(app, width, height);
    let buffer = terminal.backend().buffer().clone();
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol().to_owned())
                .collect()
        })
        .collect()
}

/// The rendered screen at the default size.
pub fn screen(app: &App) -> Vec<String> {
    screen_at(app, WIDTH, HEIGHT)
}

/// The first row containing `text`, for a test that then inspects a cell.
pub fn row(app: &App, text: &str) -> Option<usize> {
    screen(app).iter().position(|line| line.contains(text))
}
