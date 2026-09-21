//! What a host says about a data source configuration, and when it says it.
//!
//! A message is a warning or a note (its tone is [`Message`]'s) that only applies
//! in *some* browser contexts: "this browser can clear its storage" is true where
//! OPFS exists, and "everything is lost when this tab closes" is true where it
//! does not. So a message is plain data plus a predicate over [`DataSourceContext`]
//! — what the host knows about the browser it is running in — and the rules stay
//! testable without a browser.
//!
//! It lives apart from the configuration that carries the list
//! ([`DataSourceConfiguration::message`](super::data_source_configuration::DataSourceConfiguration))
//! because the predicate is the whole of the idea, and it is not about data
//! sources.

use super::data_source_configuration::DataSourceConfiguration;
use super::message::Message;

/// A message that only applies in some browser contexts, so a host can say the
/// right thing about a configuration in the browser it is actually running in.
/// Plain data plus a predicate — the rules stay testable without a browser.
#[derive(Debug, Clone)]
pub struct DataSourceMessage {
    /// Whether this message applies in the given context.
    pub applies: fn(&DataSourceContext) -> bool,
    pub message: Message,
}

// Equality ignores nothing and invents nothing: the text must match, and two
// distinct predicates are different. (A derived impl is impossible because a
// fn pointer is not `PartialEq`; the address is the honest comparison.)
impl PartialEq for DataSourceMessage {
    fn eq(&self, other: &Self) -> bool {
        self.message == other.message && self.applies as usize == other.applies as usize
    }
}

impl Eq for DataSourceMessage {}

/// What the host knows about the browser it is running in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataSourceContext {
    /// Whether the OPFS store is available here. False in a private window,
    /// and in a browser without the sync access handle (Safari).
    pub opfs_available: bool,
}

/// The configuration's messages that apply in `ctx`, in declaration order.
pub fn messages<'a>(
    configuration: &'a DataSourceConfiguration,
    ctx: &DataSourceContext,
) -> impl Iterator<Item = &'a Message> {
    let ctx = *ctx;
    configuration
        .message
        .iter()
        .filter(move |m| (m.applies)(&ctx))
        .map(|m| &m.message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Availability;
    use std::collections::BTreeMap;

    fn browser(messages: Vec<DataSourceMessage>) -> DataSourceConfiguration {
        DataSourceConfiguration {
            id: "browser",
            name: "Browser",
            description: "On this machine.",
            availability: Availability::Enabled,
            message: messages,
            metadata: BTreeMap::new(),
            fields: &[],
        }
    }

    #[test]
    fn a_message_only_applies_in_its_context() {
        let configuration = browser(vec![
            DataSourceMessage {
                applies: |c| !c.opfs_available,
                message: Message::warning("Lost on close", "No OPFS store."),
            },
            DataSourceMessage {
                applies: |c| c.opfs_available,
                message: Message::warning("Can be cleared", "Clearing site data empties it."),
            },
        ]);

        let titles = |opfs_available| {
            messages(&configuration, &DataSourceContext { opfs_available })
                .map(|m| m.title().unwrap_or_default().to_string())
                .collect::<Vec<_>>()
        };

        assert_eq!(titles(true), vec!["Can be cleared"]);
        assert_eq!(titles(false), vec!["Lost on close"]);
    }

    #[test]
    fn a_configuration_can_have_no_message() {
        let ctx = DataSourceContext {
            opfs_available: true,
        };
        let with = browser(vec![DataSourceMessage {
            applies: |c| c.opfs_available,
            message: Message::warning("Heads up", "Can be cleared."),
        }]);

        assert_eq!(messages(&with, &ctx).count(), 1);
        assert_eq!(messages(&browser(vec![]), &ctx).count(), 0);
    }
}
