//! A **storage**: one place todos are kept (ADR-0018).
//!
//! Created from a
//! [`StorageConfiguration`](super::storage_configuration::StorageConfiguration) —
//! or existing already: the default storages do, and the Browser store is a
//! storage with nothing configured and no remote. A storage has an id of its own
//! (the local db is named after it, so editing the remote keeps *this* store's
//! todos), the kind it was created from, the name the user gave it, and its field
//! values.
//!
//! The user picks a storage, never a path — the directory and the db file name are
//! the app's, and a storage's field values are the only thing about it the user
//! supplies.

use std::collections::BTreeMap;

/// A storage, as data — and as it is persisted: this is what a saved list holds,
/// and what a transport, a store path and every name a host shows derive from.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Storage {
    /// Stable and generated. The local store is named after it, so editing the
    /// remote later keeps *this* storage's todos (ADR-0017).
    pub id: String,
    /// Which configuration this storage was created from: a
    /// [`StorageConfiguration`](super::storage_configuration::StorageConfiguration)
    /// id (`opfs`, `webdav`, `s3`). It is what the app looks a storage's fields and
    /// transport up by, so it never changes.
    pub kind: String,
    /// What the user calls it. **Required** — it is the only name this storage has,
    /// wherever it is named, and a chain of fallbacks to the URL, then to the kind,
    /// was three ways to say the same thing badly. Creating one asks for it;
    /// migration names what predates that.
    pub name: String,
    /// The configured fields, keyed by `remote::keys`.
    pub fields: BTreeMap<String, String>,
}

impl Storage {
    /// What identifies the **remote this points at** — its root, never the name
    /// the user gave it. Two storages with the same identity are the same data;
    /// when this changes the remote changed, and the user has to decide what
    /// happens to the two sets of todos (ADR-0018). The caller supplies the root
    /// because only the app knows which fields address a given kind.
    pub fn identity(&self, root: &str) -> String {
        format!("{}:{}", self.kind, root.trim())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn storage(name: &str, url: &str) -> Storage {
        let mut fields = BTreeMap::new();
        fields.insert("url".to_string(), url.to_string());
        Storage {
            id: "webdav-1a2b3c4d".to_string(),
            kind: "webdav".to_string(),
            name: name.to_string(),
            fields,
        }
    }

    /// The name is metadata: renaming a storage must not look like pointing it
    /// somewhere else, or every rename would ask the user to reconcile two sets of
    /// todos that never diverged.
    #[test]
    fn identity_ignores_the_name_and_follows_the_root() {
        let home = storage("Home", "https://cloud.example.com/dav/todo");
        let renamed = storage("Laptop", "https://cloud.example.com/dav/todo");
        assert_eq!(home.identity("/dav/todo"), renamed.identity("/dav/todo"));
        // Pointing at another root is exactly a different remote.
        assert_ne!(home.identity("/dav/todo"), home.identity("/dav/other"));
        // And two kinds at the same root are two storages.
        let mut s3 = home.clone();
        s3.kind = "s3".to_string();
        assert_ne!(home.identity("/x"), s3.identity("/x"));
    }
}
