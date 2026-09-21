//! A **data source configuration**: the template a data source is created from.
//!
//! Browser, WebDAV, S3, Drive — each is plain data: what a data source of that
//! kind needs (its `fields`), what to call it, what to say about it (`message`,
//! per browser context) and whether the host offers it at all (`availability`).
//! The lib does not know which ones exist: the host configures the list
//! (`app/constants.rs`), and offers the ones it declares.
//!
//! The other half of this pair is [`DataSource`](super::data_source::DataSource):
//! one place todos are kept, created from one of these — or, for the default data
//! sources, existing already.
//!
//! A configuration's `message` list says what the app wants the user to know about
//! it (why it is not durable, …), per browser context — see
//! [`data_source_message`](super::data_source_message).

use std::collections::BTreeMap;

use super::data_source_message::DataSourceMessage;
use super::form::Field;

/// Whether this app offers a configuration: the host's control over its options.
///
/// Not a statement about the backend. An app offers the configurations it was
/// built for and leaves out the rest — a backend nobody has finished, one this
/// build has no wiring for (Drive without an OAuth client), or one the product
/// does not want. That the decision belongs to the host is what makes the list
/// polymorphic: one model serves an app with two options and an app with five,
/// and nothing else in the lib changes. A configuration that is not offered is
/// still a configuration — looking it up by id finds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    /// Offered: a choice the user can make, and a data source can be created from
    /// it. What a host does with that — this app leaves the rest out of the list
    /// it offers — is the host's business, not the model's.
    Enabled,
    /// Not offered by this app. Drive is the current case: its sign-in flow cannot
    /// be verified end to end, so this build does not offer it (ADR-0015).
    Disabled,
}

/// A data source configuration, as data: the template a data source is created
/// from. The host declares the ones it offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataSourceConfiguration {
    /// Stable id: what a host refers to this configuration by, and stores a
    /// preference under. It is also how a [`DataSource`](super::data_source::DataSource)
    /// in the app says which configuration it was created from.
    pub id: &'static str,
    /// What this configuration is called (Browser, WebDAV, S3) — a name, not a
    /// label: how the user sees it is the host's business.
    pub name: &'static str,
    /// What this kind of data source is, in one sentence, for the user.
    pub description: &'static str,
    pub availability: Availability,
    /// What to tell the user about this data source, when it applies.
    pub message: Vec<DataSourceMessage>,
    /// Structured detail about the backend (its folder, its provider, …).
    /// Empty when the data source has nothing to say beyond the fields above.
    pub metadata: BTreeMap<String, String>,
    /// Configuration the user must supply before this data source can be used
    /// (a WebDAV or S3 endpoint, credentials). Empty when there is nothing to
    /// configure — the Browser store, and Drive (whose configuration is the
    /// OAuth grant). See ADR-0016.
    pub fields: &'static [Field],
}

impl DataSourceConfiguration {
    /// The configuration with this id, or `None`.
    pub fn from_id<'a>(
        configurations: &'a [DataSourceConfiguration],
        id: &str,
    ) -> Option<&'a DataSourceConfiguration> {
        configurations.iter().find(|s| s.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Message;

    fn sample() -> Vec<DataSourceConfiguration> {
        vec![
            DataSourceConfiguration {
                id: "local",
                name: "Browser",
                description: "On this machine.",
                availability: Availability::Enabled,
                message: vec![DataSourceMessage {
                    applies: |c| c.opfs_available,
                    message: Message::warning("Heads up", "Can be cleared."),
                }],
                metadata: BTreeMap::new(),
                fields: &[],
            },
            DataSourceConfiguration {
                id: "remote",
                name: "Remote",
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
            DataSourceConfiguration::from_id(&list, "local").map(|s| s.name),
            Some("Browser")
        );
        assert_eq!(DataSourceConfiguration::from_id(&list, "nope"), None);
    }
}
