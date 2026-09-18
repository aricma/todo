//! Where todos live (MVP.md §4): a storage backend the app can keep todos in.
//!
//! `Storage` is plain data — id, label, description, availability, message.
//! The lib does not know which storages exist: the app configures the options
//! it offers (`app/constants.rs`) and the view renders the enabled ones.
//!
//! The user picks a **storage**, never a path — the directory and the db file
//! name are the app's. A storage's `message` carries what the app wants the
//! user to know about it (why it is not durable, …), in a tone the view draws.

use std::collections::BTreeMap;

use super::message::Message;

/// Whether a storage can be used in the app today.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    /// Shown and selectable: the backend exists and works.
    Enabled,
    /// Not shown: the backend is planned but does not work yet.
    Disabled,
}

/// A storage backend, as data. The app configures the options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Storage {
    /// Stable id — the radio group's value and the stored preference. The app
    /// also uses it to tell backends apart (e.g. the syncing one).
    pub id: &'static str,
    /// The user-facing name.
    pub label: &'static str,
    /// The verb for the start screen's primary action on this storage
    /// ("Open", "Connect", …).
    pub action: &'static str,
    /// What the storage is, shown under the tiles.
    pub description: &'static str,
    pub availability: Availability,
    /// What to tell the user about this storage, if anything.
    pub message: Option<Message>,
    /// Structured detail about the backend (its folder, its provider, …).
    /// Empty when the storage has nothing to say beyond the fields above.
    pub metadata: BTreeMap<String, String>,
}

impl Storage {
    /// The storage with this id, or `None`. Ids are the app's vocabulary.
    pub fn from_id<'a>(storages: &'a [Storage], id: &str) -> Option<&'a Storage> {
        storages.iter().find(|s| s.id == id)
    }
}

/// The storages the app should offer, in display order.
pub fn enabled(storages: &[Storage]) -> impl Iterator<Item = &Storage> {
    storages
        .iter()
        .filter(|s| s.availability == Availability::Enabled)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<Storage> {
        vec![
            Storage {
                id: "local",
                label: "Browser",
                action: "Open",
                description: "On this machine.",
                availability: Availability::Enabled,
                message: Some(Message::warning("Heads up", "Can be cleared.")),
                metadata: BTreeMap::new(),
            },
            Storage {
                id: "remote",
                label: "Remote",
                action: "Connect",
                description: "Somewhere else.",
                availability: Availability::Disabled,
                message: None,
                metadata: BTreeMap::new(),
            },
        ]
    }

    #[test]
    fn from_id_finds_or_returns_none() {
        let list = sample();
        assert_eq!(
            Storage::from_id(&list, "local").map(|s| s.label),
            Some("Browser")
        );
        assert_eq!(Storage::from_id(&list, "nope"), None);
    }

    #[test]
    fn enabled_filters_out_disabled() {
        let list = sample();
        let ids: Vec<&str> = enabled(&list).map(|s| s.id).collect();
        assert_eq!(ids, vec!["local"]);
    }

    #[test]
    fn a_storages_message_is_optional() {
        let list = sample();
        assert!(Storage::from_id(&list, "local").unwrap().message.is_some());
        assert!(Storage::from_id(&list, "remote").unwrap().message.is_none());
    }
}
