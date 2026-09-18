//! Sync state over the local store (ADR-0015). Pure and wasm-free: the
//! browser half (`app/side_effects/sync.rs`) calls these rules and reports
//! their results. The only concurrency token is Drive's `version` field —
//! Drive v3 does not honour `If-Match`, so an ETag is not an option.

/// What the sync layer is doing / waiting on. Rendered by the shell and the
/// Settings → Data source card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncState {
    /// The OPFS provider is selected: there is nothing to sync.
    Disabled,
    /// Drive is selected but this browser has never signed in.
    NeedsSignIn,
    /// Signed in, connecting: finding the folder and file.
    Connecting,
    Pulling,
    Pushing,
    /// Connected and level with the remote.
    Idle,
    /// The remote changed since `last_version` while a local change was
    /// pending. Nothing is overwritten until the user chooses a side.
    Conflict {
        remote_version: String,
    },
    Error(String),
}

/// What a push should do, given what the device last saw and what the remote
/// says now. `remote_version` is always known (the caller fetched it);
/// `last_version` is `None` on a first connect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushDecision {
    /// Safe to upload: the remote is exactly where this device left it.
    Push,
    /// The remote moved on. Do not touch it.
    Conflict,
}

/// The version guard. `None` is a device that has never synced — for a file
/// that exists that is a conflict (another device created it), because
/// pushing would clobber whichever device got there first.
pub fn push_decision(last_version: Option<&str>, remote_version: &str) -> PushDecision {
    match last_version {
        Some(v) if v == remote_version => PushDecision::Push,
        _ => PushDecision::Conflict,
    }
}

/// Whether a boot may skip the download: the remote is at the version this
/// device last synced **and** there are no local edits waiting.
pub fn can_skip_pull(last_version: Option<&str>, remote_version: &str, dirty: bool) -> bool {
    !dirty && matches!(last_version, Some(v) if v == remote_version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_versions_push() {
        assert_eq!(push_decision(Some("7"), "7"), PushDecision::Push);
    }

    #[test]
    fn a_remote_that_moved_on_is_a_conflict() {
        assert_eq!(push_decision(Some("7"), "8"), PushDecision::Conflict);
    }

    /// First connect over an existing remote file: pushing would overwrite
    /// data this device has never seen.
    #[test]
    fn no_remembered_version_is_a_conflict_not_a_push() {
        assert_eq!(push_decision(None, "1"), PushDecision::Conflict);
    }

    #[test]
    fn skip_pull_only_when_level_and_clean() {
        assert!(can_skip_pull(Some("3"), "3", false));
        assert!(!can_skip_pull(Some("3"), "3", true));
        assert!(!can_skip_pull(Some("3"), "4", false));
        assert!(!can_skip_pull(None, "3", false));
    }
}
