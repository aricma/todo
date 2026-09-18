//! Where todos live (MVP.md §4): a storage backend the app can keep todos in.
//!
//! `Storage` is plain data — id, label, description, availability, messages,
//! configuration fields. The lib does not know which storages exist: the app
//! configures the options it offers (`app/constants.rs`) and the view renders
//! the enabled ones.
//!
//! The user picks a **storage**, never a path — the directory and the db file
//! name are the app's. A storage's `message` carries what the app wants the
//! user to know about it (why it is not durable, …), in a tone the view draws;
//! each message says which browser contexts it applies in, so the same storage
//! can warn about clearing and about losing everything on close.

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
    /// What to tell the user about this storage, when it applies.
    pub message: Vec<StorageMessage>,
    /// Structured detail about the backend (its folder, its provider, …).
    /// Empty when the storage has nothing to say beyond the fields above.
    pub metadata: BTreeMap<String, String>,
    /// Configuration the user must supply before this storage can be used
    /// (a WebDAV or S3 endpoint, credentials). Empty when there is nothing to
    /// configure — the Browser store, and Drive (whose configuration is the
    /// OAuth grant). See ADR-0016.
    pub fields: &'static [StorageField],
}

/// A message that only applies in some browser contexts, so the start screen
/// can say the right thing about a storage in the browser it is actually
/// running in. Plain data plus a predicate — the rules stay testable without
/// a browser.
#[derive(Debug, Clone)]
pub struct StorageMessage {
    /// Whether this message applies in the given context.
    pub applies: fn(&StorageContext) -> bool,
    pub message: Message,
}

// Equality ignores nothing and invents nothing: the text must match, and two
// distinct predicates are different. (A derived impl is impossible because a
// fn pointer is not `PartialEq`; the address is the honest comparison.)
impl PartialEq for StorageMessage {
    fn eq(&self, other: &Self) -> bool {
        self.message == other.message && self.applies as usize == other.applies as usize
    }
}

impl Eq for StorageMessage {}

/// What the app knows about the browser it is running in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StorageContext {
    /// Whether there is a persistent local store here (OPFS). False in a
    /// private window, and in a browser without the sync access handle.
    pub persistent: bool,
}

/// How a storage's configuration field is drawn and validated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    Text,
    Password,
    Url,
    Number,
}

/// One configuration field a storage needs before it can be used. The view
/// renders a form from these; the app stores the values against the storage
/// id, never in the model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageField {
    /// Stable key the value is stored under (`endpoint`, `secret_key`, …).
    pub key: &'static str,
    pub label: &'static str,
    pub kind: FieldKind,
    /// Whether the storage cannot be used until this has a value.
    pub required: bool,
    pub placeholder: Option<&'static str>,
}

/// The storage's messages that apply in `ctx`, in declaration order.
pub fn messages<'a>(
    storage: &'a Storage,
    ctx: &StorageContext,
) -> impl Iterator<Item = &'a Message> {
    let ctx = *ctx;
    storage
        .message
        .iter()
        .filter(move |m| (m.applies)(&ctx))
        .map(|m| &m.message)
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
                message: vec![StorageMessage {
                    applies: |c| c.persistent,
                    message: Message::warning("Heads up", "Can be cleared."),
                }],
                metadata: BTreeMap::new(),
                fields: &[],
            },
            Storage {
                id: "remote",
                label: "Remote",
                action: "Connect",
                description: "Somewhere else.",
                availability: Availability::Disabled,
                message: vec![],
                metadata: BTreeMap::new(),
                fields: &[],
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
    fn a_message_only_applies_in_its_context() {
        let storage = Storage {
            id: "browser",
            label: "Browser",
            action: "Open",
            description: "On this machine.",
            availability: Availability::Enabled,
            message: vec![
                StorageMessage {
                    applies: |c| !c.persistent,
                    message: Message::warning("Lost on close", "No persistent store."),
                },
                StorageMessage {
                    applies: |c| c.persistent,
                    message: Message::warning("Can be cleared", "Clearing site data empties it."),
                },
            ],
            metadata: BTreeMap::new(),
            fields: &[],
        };

        let titles = |persistent| {
            messages(&storage, &StorageContext { persistent })
                .map(|m| m.title().unwrap_or_default().to_string())
                .collect::<Vec<_>>()
        };

        assert_eq!(titles(true), vec!["Can be cleared"]);
        assert_eq!(titles(false), vec!["Lost on close"]);
    }

    #[test]
    fn a_storage_can_have_no_message() {
        let list = sample();
        let ctx = StorageContext { persistent: true };
        assert_eq!(
            messages(Storage::from_id(&list, "local").unwrap(), &ctx).count(),
            1
        );
        assert_eq!(
            messages(Storage::from_id(&list, "remote").unwrap(), &ctx).count(),
            0
        );
    }
}
