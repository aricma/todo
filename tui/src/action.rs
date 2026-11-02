//! What a key press asks the terminal layer to do.

use std::path::PathBuf;

/// Which section has the cursor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pane {
    Files,
    /// The document's headings.
    Outline,
    /// The file's own text, on the right. Only the sideways arrows reach it;
    /// `Tab` never does.
    View,
    /// The todos list, which only a single-file session shows.
    Todos,
    History,
}

impl Pane {
    /// What the status bar calls this section.
    pub fn name(self) -> &'static str {
        match self {
            Self::Files => "files",
            Self::Outline => "outline",
            Self::View => "view",
            Self::Todos => "todos",
            Self::History => "history",
        }
    }
}

/// One effect the app cannot perform itself, because it touches the disk or the
/// terminal. `on_key` returns it; the caller does it and reports back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Save { path: PathBuf, text: String },
    Reload,
    Quit,
}
