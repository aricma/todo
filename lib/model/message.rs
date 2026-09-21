//! A tone-coded message (DESIGN.md §7): what a provider wants to tell the user,
//! without prescribing how it is drawn. The host picks a tone
//! from the variant and renders the parts: an optional title, the text itself,
//! and optional metadata (structured detail a host may show or log).
//!
//! Deliberately data, not markup: the lib owns the words and the tone, the UI
//! owns the presentation.

use std::collections::BTreeMap;

/// A message in one of the four tones. The text is required; the title and the
/// metadata are optional.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Info {
        title: Option<String>,
        text: String,
        metadata: Option<BTreeMap<String, String>>,
    },
    Success {
        title: Option<String>,
        text: String,
        metadata: Option<BTreeMap<String, String>>,
    },
    Warning {
        title: Option<String>,
        text: String,
        metadata: Option<BTreeMap<String, String>>,
    },
    Error {
        title: Option<String>,
        text: String,
        metadata: Option<BTreeMap<String, String>>,
    },
}

impl Message {
    pub fn info(title: impl Into<String>, text: impl Into<String>) -> Message {
        Message::Info {
            title: Some(title.into()),
            text: text.into(),
            metadata: None,
        }
    }

    pub fn success(title: impl Into<String>, text: impl Into<String>) -> Message {
        Message::Success {
            title: Some(title.into()),
            text: text.into(),
            metadata: None,
        }
    }

    pub fn warning(title: impl Into<String>, text: impl Into<String>) -> Message {
        Message::Warning {
            title: Some(title.into()),
            text: text.into(),
            metadata: None,
        }
    }

    pub fn error(title: impl Into<String>, text: impl Into<String>) -> Message {
        Message::Error {
            title: Some(title.into()),
            text: text.into(),
            metadata: None,
        }
    }

    /// Attach structured detail (what was skipped, per row; a status code; …)
    /// to any tone.
    pub fn with_metadata(mut self, metadata: BTreeMap<String, String>) -> Message {
        match &mut self {
            Message::Info { metadata: m, .. }
            | Message::Success { metadata: m, .. }
            | Message::Warning { metadata: m, .. }
            | Message::Error { metadata: m, .. } => *m = Some(metadata),
        }
        self
    }

    pub fn title(&self) -> Option<&str> {
        match self {
            Message::Info { title, .. }
            | Message::Success { title, .. }
            | Message::Warning { title, .. }
            | Message::Error { title, .. } => title.as_deref(),
        }
    }

    pub fn text(&self) -> &str {
        match self {
            Message::Info { text, .. }
            | Message::Success { text, .. }
            | Message::Warning { text, .. }
            | Message::Error { text, .. } => text,
        }
    }

    pub fn metadata(&self) -> Option<&BTreeMap<String, String>> {
        match self {
            Message::Info { metadata, .. }
            | Message::Success { metadata, .. }
            | Message::Warning { metadata, .. }
            | Message::Error { metadata, .. } => metadata.as_ref(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_and_accessors_round_trip() {
        let m = Message::warning("Heads up", "DataSourceConfiguration can be cleared.");
        assert_eq!(m.title(), Some("Heads up"));
        assert_eq!(m.text(), "DataSourceConfiguration can be cleared.");
        assert_eq!(m.metadata(), None);
        assert!(matches!(m, Message::Warning { .. }));
    }

    #[test]
    fn the_title_is_optional() {
        let bare = Message::Info {
            title: None,
            text: "Saved.".to_string(),
            metadata: None,
        };
        assert_eq!(bare.title(), None);
        assert_eq!(bare.text(), "Saved.");
    }

    #[test]
    fn metadata_is_optional_and_attaches_to_any_tone() {
        assert_eq!(Message::success("Done", "Saved").metadata(), None);

        let mut metadata = BTreeMap::new();
        metadata.insert("rows skipped".to_string(), "2".to_string());
        let with = Message::error("Import failed", "2 rows skipped").with_metadata(metadata);
        assert_eq!(with.metadata().map(|m| m.len()), Some(1));
        assert_eq!(with.text(), "2 rows skipped");
    }
}
