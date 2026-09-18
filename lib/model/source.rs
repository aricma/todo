//! Where todos live (MVP.md §4): the one source this app opens, and the
//! providers it could live in.
//!
//! The user picks a **provider**, never a path — the directory and the db file
//! name are ours. That holds for the future too: OPFS is the local store, and
//! when a syncing provider lands it becomes that provider's cache, so the
//! selection stays meaningful while the path stays an implementation detail.
//!
//! The storage layout itself (the fixed directory and db file name) is an
//! app-layer constant (`app/constants.rs`), where the other identifiers the
//! bin needs live.
//!
//! The table below is the whole list; `DataSourcePanel` renders it in both
//! places it appears (the start screen and Settings), so the options are
//! stated once. A provider declares what it is called, what it does and
//! whether it can be selected; the view owns what it looks like (its icon) and
//! how the one that is implemented is configured (its rows), because those are
//! markup rather than data.

use super::message::Message;

/// Whether a provider can be selected today.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    /// Selectable: the backend exists and works.
    Enabled,
    /// Listed but not selectable: picking it would do nothing yet.
    Locked,
}

/// A backend todos can live in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    /// This browser's private file storage (OPFS) — the local store.
    Opfs,
    /// A Google Drive folder, synced (planned; OPFS becomes its cache).
    GoogleDrive,
}

impl Provider {
    /// The provider opened when nothing (or garbage) is stored: a fresh
    /// browser starts local, and the user opts into anything else explicitly.
    pub const DEFAULT: Provider = Provider::Opfs;

    /// Every provider, in display order.
    pub const ALL: [Provider; 2] = [Provider::Opfs, Provider::GoogleDrive];

    /// Stable id — the radio group's value, the stored preference and the
    /// group's identity. The technical identity, not the label.
    pub fn id(self) -> &'static str {
        match self {
            Provider::Opfs => "opfs",
            Provider::GoogleDrive => "google-drive",
        }
    }

    /// Stored value → provider. Unknown values are `None`: the caller falls
    /// back to [`DEFAULT`] rather than guessing.
    pub fn from_id(id: &str) -> Option<Provider> {
        match id {
            "opfs" => Some(Provider::Opfs),
            "google-drive" => Some(Provider::GoogleDrive),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Provider::Opfs => "Browser",
            Provider::GoogleDrive => "Google Drive",
        }
    }

    /// What the provider is, shown for the selected one (and for the locked
    /// ones, so a locked tile is not a mystery).
    pub fn description(self) -> &'static str {
        match self {
            Provider::Opfs => {
                "Todos are stored in this browser, on this machine. Nothing is uploaded, \
                 and they are still here after you close the tab."
            }
            Provider::GoogleDrive => {
                "Keep todos in a Google Drive folder and sync them between your devices."
            }
        }
    }

    pub fn availability(self) -> Availability {
        match self {
            Provider::Opfs => Availability::Enabled,
            Provider::GoogleDrive => Availability::Locked,
        }
    }

    /// What the provider wants the user to know, or `None` when there is
    /// nothing to say. The one place the words and the tone are written, so
    /// the tile, the start screen and Settings cannot disagree — and data
    /// rather than markup, so a view picks the frame. Generic on purpose: more
    /// providers are coming, so it points at "a different data source", never
    /// at one by name.
    pub fn message(self) -> Option<Message> {
        match self {
            Provider::Opfs => Some(Message::warning(
                "Storage can be cleared",
                "This browser can clear its storage at any time — clearing site data, \
                 an incognito window or a new device starts empty. Choose a different \
                 data source if you want the todos to last.",
            )),
            Provider::GoogleDrive => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The radio group needs exactly one option it can land on: a list where
    /// everything is locked, or where two providers claim to be the selected
    /// one, is a bug in the table rather than in the view.
    #[test]
    fn exactly_one_provider_is_selectable() {
        let enabled = Provider::ALL
            .iter()
            .filter(|p| p.availability() == Availability::Enabled)
            .count();
        assert_eq!(enabled, 1);
        assert_eq!(Provider::DEFAULT.availability(), Availability::Enabled);
    }

    /// The stored preference round-trips; unknown/garbage values fall back to
    /// the default instead of leaving the app with no provider.
    #[test]
    fn provider_id_round_trips_and_falls_back() {
        for p in Provider::ALL {
            assert_eq!(Provider::from_id(p.id()), Some(p));
        }
        assert_eq!(Provider::from_id("dropbox"), None);
        assert_eq!(Provider::from_id(""), None);
    }

    /// Ids are the radio group's values and its `name` — duplicates would
    /// make two tiles one option.
    /// The user-facing name is Browser while the technical variant/key stay
    /// Opfs — the storage engine is the developer's word, and localStorage or
    /// IndexedDB would be Browser to the user too.
    #[test]
    fn the_local_provider_is_called_browser() {
        assert_eq!(Provider::Opfs.label(), "Browser");
        assert_eq!(Provider::Opfs.id(), "opfs");
    }

    /// Only the browser-backed provider warns that its storage is not
    /// durable; the durable option must not carry the warning.
    #[test]
    fn only_the_browser_provider_warns_about_cleared_storage() {
        let msg = Provider::Opfs.message().expect("Browser message");
        assert!(matches!(msg, Message::Warning { .. }));
        let text = msg.text();
        assert!(text.contains("clear"), "{text}");
        assert!(text.contains("data source"), "{text}");
        assert!(
            !text.contains("Google"),
            "names a specific provider: {text}"
        );
        assert_eq!(Provider::GoogleDrive.message(), None);
    }

    #[test]
    fn provider_ids_and_labels_are_unique() {
        let mut ids: Vec<&str> = Provider::ALL.iter().map(|p| p.id()).collect();
        let mut labels: Vec<&str> = Provider::ALL.iter().map(|p| p.label()).collect();
        ids.sort_unstable();
        labels.sort_unstable();
        let unique_ids = ids.len();
        let unique_labels = labels.len();
        ids.dedup();
        labels.dedup();
        assert_eq!(ids.len(), unique_ids);
        assert_eq!(labels.len(), unique_labels);
    }

    #[test]
    fn every_provider_says_what_it_is() {
        for p in Provider::ALL {
            assert!(!p.label().is_empty());
            assert!(
                p.description().len() > 30,
                "{} needs a real description",
                p.label()
            );
        }
    }
}
