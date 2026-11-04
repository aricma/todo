//! What the app says about an outcome, and how much it matters.
//!
//! Not a **hint**: a hint is the reminder of a key — the status bar and the keys
//! page, no tone, true until the layout changes. A **message** is about something
//! that just happened: a rule being explained, a write refused, nothing found. It
//! carries a tone, because "worth knowing" and "did not happen" must not look the
//! same.
//!
//! A message about one row lives on that row (`Entry::message`); a message about
//! the panel it is drawn in lives in `App`. Nothing carries an index into somebody
//! else's list — an index goes stale the moment a row is removed, and the row that
//! owns its message cannot.

/// How much a message matters. The only thing it decides is the colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    /// Nothing is wrong and nothing failed; there is simply nothing to report.
    Info,
    /// A rule being explained. Nothing has failed, so nothing wears the colour of
    /// a failure.
    Warning,
    /// A write that did not happen.
    Error,
    /// Something worked, and the reader should see that it did: a copy that landed.
    Success,
}

/// One line about what just happened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub tone: Tone,
    pub text: String,
}

impl Message {
    /// Whether this is the kind the header carries.
    pub const fn is_success(&self) -> bool {
        matches!(self.tone, Tone::Success)
    }

    /// For the reader's information: "no todo matches", "8 files searched".
    pub fn info(text: impl Into<String>) -> Self {
        Self {
            tone: Tone::Info,
            text: text.into(),
        }
    }

    /// An answer to "why can't I do that?": a rule, not a failure.
    pub fn warning(text: impl Into<String>) -> Self {
        Self {
            tone: Tone::Warning,
            text: text.into(),
        }
    }

    /// A write that did not happen.
    pub fn failed(text: impl Into<String>) -> Self {
        Self {
            tone: Tone::Error,
            text: text.into(),
        }
    }

    /// Something that worked. It is drawn in the header rather than beside the
    /// thing it is about: nothing is wrong, so nothing on the panel needs to move
    /// over to say so.
    pub fn success(text: impl Into<String>) -> Self {
        Self {
            tone: Tone::Success,
            text: text.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Message, Tone};

    #[test]
    fn only_a_failure_is_an_error() {
        assert_eq!(Message::info("x").tone, Tone::Info);
        assert_eq!(Message::warning("x").tone, Tone::Warning);
        assert_eq!(Message::failed("x").tone, Tone::Error);
        assert_eq!(Message::success("x").tone, Tone::Success);
    }
}
