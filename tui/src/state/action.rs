//! What a key press asks the terminal layer to do.

use crate::text;

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
    /// The todos of one tag or person, gathered from every file they are in. Its
    /// keys are the ones that make sense for a todo with no file of its own.
    Collection,
    History,
}

impl Pane {
    /// Whether the sidebar holds this section. `tab` and `opt+↑/↓` move between
    /// these; the file view is off to the side and `←` reaches it.
    pub const fn in_sidebar(self) -> bool {
        matches!(self, Self::Files | Self::Outline | Self::History)
    }

    /// What the status bar calls this section.
    pub fn name(self) -> &'static str {
        match self {
            Self::Files => text::FILES,
            Self::Outline => text::INDEX,
            Self::View => text::VIEW,
            Self::Todos => text::TODOS,
            Self::Collection => text::COLLECTION,
            Self::History => text::HISTORY,
        }
    }
}

/// One effect the app cannot perform itself, because it touches the disk or the
/// terminal. `on_key` returns it; the caller does it and reports back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// Write this text over whatever the file says now: the resolution of a conflict,
    /// chosen by a person who was shown both sides.
    Overwrite {
        text: String,
    },
    /// Put this on the terminal's clipboard (OSC 52). The terminal layer is the only
    /// one that can write to it, and not every terminal listens.
    Copy {
        text: String,
    },
    /// Write this text to the file the app has open. The path is the app's, not
    /// the action's: the caller saves what it loaded, so the two cannot disagree.
    Save {
        text: String,
    },
    Quit,
}
