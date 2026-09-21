//! Sync state over the local store (ADR-0015, ADR-0016). Pure and wasm-free:
//! the browser half (`app/side_effects/sync.rs`) calls these rules and reports
//! their results.
//!
//! The concurrency token is per transport: Drive's `version` field (v3 does
//! not honour `If-Match`, so an ETag is not an option there), and WebDAV/S3's
//! `ETag` — which those transports send back as `If-Match`, making the check
//! and the write one atomic request.

/// What the sync layer is doing / waiting on, for a host to draw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataSourceSyncState {
    /// The OPFS provider is selected: there is nothing to sync.
    Disabled,
    /// Drive is selected but this browser has never signed in.
    NeedsSignIn,
    /// A self-hosted remote (WebDAV, S3) is selected and its configuration is
    /// incomplete. Distinct from `NeedsSignIn`: there is no grant to wait for,
    /// only fields to fill in.
    NeedsConfig,
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
    /// The configuration now points at a **different remote** than the one this
    /// source's todos came from. Renaming is not this; pointing elsewhere is, and
    /// the user decides which side wins and whether to merge (ADR-0018). Nothing
    /// is read or written until then.
    Error(String),
}

/// One word for this state: the host pairs it with the storage's name, where
/// there is no room for a sentence. Every state says something: "nothing to sync"
/// and "waiting for a grant" are as worth showing as "pushing".
pub fn status_label(state: &DataSourceSyncState) -> &'static str {
    match state {
        DataSourceSyncState::Disabled => "in this browser",
        DataSourceSyncState::NeedsSignIn => "sign in",
        DataSourceSyncState::NeedsConfig => "settings needed",
        DataSourceSyncState::Connecting => "connecting…",
        DataSourceSyncState::Pulling => "pulling…",
        DataSourceSyncState::Pushing => "pushing…",
        DataSourceSyncState::Idle => "synced",
        DataSourceSyncState::Conflict { .. } => "conflict",
        DataSourceSyncState::Error(_) => "error",
    }
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
        Some(v) if same_version(v, remote_version) => PushDecision::Push,
        _ => PushDecision::Conflict,
    }
}

/// Whether a boot may skip the download: the remote is at the version this
/// device last synced **and** there are no local edits waiting.
pub fn can_skip_pull(last_version: Option<&str>, remote_version: &str, dirty: bool) -> bool {
    !dirty && matches!(last_version, Some(v) if same_version(v, remote_version))
}

/// Whether two validators name the same version, comparing opaque tags the way
/// RFC 7232 defines them: the weakness indicator (`W/`) is **not** part of the
/// tag. Same tag = same version, whether the server chose to call it weak.
///
/// This is not pedantry. Apache answers the *same unchanged file* with
/// `W/"e000-65bda2f684b6c"` on one `HEAD` and `"e000-65bda2f684b6c"` on the next
/// (measured), so a byte comparison read a file it had just written as "the
/// remote moved on" — and refused every push after the first, silently, leaving
/// the user's edits local while the app sat on a conflict card.
fn same_version(a: &str, b: &str) -> bool {
    a.strip_prefix("W/").unwrap_or(a) == b.strip_prefix("W/").unwrap_or(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every state has something to say in the header, and the two "not ready
    /// yet" ones stay distinct there too.
    #[test]
    fn every_state_has_a_status() {
        assert_eq!(
            status_label(&DataSourceSyncState::Disabled),
            "in this browser"
        );
        assert_eq!(status_label(&DataSourceSyncState::Idle), "synced");
        assert_eq!(status_label(&DataSourceSyncState::Pushing), "pushing…");
        assert_eq!(status_label(&DataSourceSyncState::Pulling), "pulling…");
        assert_eq!(
            status_label(&DataSourceSyncState::Connecting),
            "connecting…"
        );
        assert_eq!(
            status_label(&DataSourceSyncState::NeedsConfig),
            "settings needed"
        );
        assert_ne!(
            status_label(&DataSourceSyncState::NeedsConfig),
            status_label(&DataSourceSyncState::NeedsSignIn)
        );
        assert_eq!(
            status_label(&DataSourceSyncState::Conflict {
                remote_version: "9".into()
            }),
            "conflict"
        );
        assert_eq!(
            status_label(&DataSourceSyncState::Error("x".into())),
            "error"
        );
    }

    /// The two "not ready yet" states mean different things: one waits for a
    /// grant, the other for the user to fill in fields.
    #[test]
    fn needs_config_is_not_needs_sign_in() {
        assert_ne!(
            DataSourceSyncState::NeedsConfig,
            DataSourceSyncState::NeedsSignIn
        );
    }

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

    /// Apache calls the same file's tag weak on one HEAD and strong on the next.
    /// The weakness indicator is not part of the tag, so neither direction of
    /// that flip may read as a conflict.
    #[test]
    fn the_weak_indicator_is_not_part_of_the_version() {
        assert_eq!(
            push_decision(Some("W/\"e000-65bda2f684b6c\""), "\"e000-65bda2f684b6c\""),
            PushDecision::Push
        );
        assert_eq!(
            push_decision(Some("\"e000-65bda2f684b6c\""), "W/\"e000-65bda2f684b6c\""),
            PushDecision::Push
        );
        assert!(can_skip_pull(Some("W/\"7\""), "\"7\"", false));
        // …but a different tag still is one.
        assert_eq!(
            push_decision(Some("W/\"7\""), "\"8\""),
            PushDecision::Conflict
        );
    }
}
