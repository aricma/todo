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
    /// Every provider, in display order.
    pub const ALL: [Provider; 2] = [Provider::Opfs, Provider::GoogleDrive];

    /// Stable id — the radio group's value, and the group's identity.
    pub fn id(self) -> &'static str {
        match self {
            Provider::Opfs => "opfs",
            Provider::GoogleDrive => "google-drive",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Provider::Opfs => "OPFS",
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

    /// The selectable provider — what the app opens today. The panel starts
    /// its radio group here; when a second provider is enabled this becomes
    /// the stored choice instead.
    pub fn selected() -> Provider {
        Provider::Opfs
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
        assert_eq!(Provider::selected().availability(), Availability::Enabled);
    }

    /// Ids are the radio group's values and its `name` — duplicates would
    /// make two tiles one option.
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
