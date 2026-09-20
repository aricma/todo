//! App state machine (MVP.md §4), pure and host-testable: the state model
//! plus the transition actions. The db round-trips stay in the app bin
//! (db::open_flow); this module turns their results into the next state, so
//! the machine is testable away from the UI (states-module inbox todo).
//!
//! There is one source at a fixed path (`app/constants.rs`), so no path is
//! carried in the state: what varies is whether the local store exists yet — a
//! fresh browser, or one whose data the user just deleted, has nothing to open
//! and asks; otherwise the app opens the store it already has.

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Booting,
    /// The start screen: the providers to choose from, and the reason the
    /// previous open failed when there was one.
    SourceSelect {
        warning: Option<String>,
    },
    Opening,
    Ready,
    Error(String),
}

impl AppState {
    /// Booting → Opening (the source exists) or the start screen (it does not:
    /// a fresh browser, or the data was deleted).
    pub fn boot(source_exists: bool) -> AppState {
        if source_exists {
            AppState::Opening
        } else {
            AppState::SourceSelect { warning: None }
        }
    }

    /// Opening → Ready on success; Error for sqlite-init failures (fatal,
    /// nothing to retry); back to SourceSelect with a warning for
    /// source-open/migration failures (the source may have been wiped).
    pub fn opening_finished(self, result: Result<(), String>) -> AppState {
        let AppState::Opening = self else {
            return self; // not in Opening — no transition
        };
        match result {
            Ok(()) => AppState::Ready,
            Err(e) if is_fatal_open_error(&e) => AppState::Error(e),
            Err(e) => AppState::SourceSelect { warning: Some(e) },
        }
    }

    /// Ready → the start screen (the user asked for the data source, or
    /// deleted the data).
    pub fn source_select() -> AppState {
        AppState::SourceSelect { warning: None }
    }
}

/// Which open failures are terminal. sqlite-wasm init is the only fatal one
/// — the worker is unusable, nothing to retry. Matches db::init's error
/// prefix (contract: db.rs keeps it stable).
pub fn is_fatal_open_error(e: &str) -> bool {
    // sqlite-wasm init is terminal, and so is a non-isolated page: no OPFS
    // VFS can install there, so retrying the open is pointless.
    e.starts_with("sqlite init failed") || e.starts_with("local storage unavailable")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot_with_an_existing_source_opens_it() {
        assert_eq!(AppState::boot(true), AppState::Opening);
    }

    #[test]
    fn boot_without_a_source_shows_the_start_screen() {
        assert_eq!(
            AppState::boot(false),
            AppState::SourceSelect { warning: None }
        );
    }

    #[test]
    fn opening_success_reaches_ready() {
        assert_eq!(AppState::Opening.opening_finished(Ok(())), AppState::Ready);
    }

    #[test]
    fn opening_sqlite_init_failure_is_fatal() {
        assert_eq!(
            AppState::Opening.opening_finished(Err("sqlite init failed: worker timeout".into())),
            AppState::Error("sqlite init failed: worker timeout".into())
        );
    }

    #[test]
    fn opening_source_failure_returns_to_select_with_warning() {
        assert_eq!(
            AppState::Opening.opening_finished(Err("open failed: no such file".into())),
            AppState::SourceSelect {
                warning: Some("open failed: no such file".into()),
            }
        );
    }

    #[test]
    fn opening_finished_outside_opening_is_a_noop() {
        assert_eq!(
            AppState::Booting.opening_finished(Ok(())),
            AppState::Booting
        );
        assert_eq!(AppState::Ready.opening_finished(Ok(())), AppState::Ready);
    }

    /// Deleting the data lands on the start screen, which is also where a
    /// fresh browser starts — one screen, both entries.
    #[test]
    fn the_start_screen_is_reachable_from_ready_and_from_boot() {
        assert_eq!(AppState::source_select(), AppState::boot(false));
    }

    #[test]
    fn fatal_error_classification() {
        assert!(is_fatal_open_error("sqlite init failed: boom"));
        assert!(is_fatal_open_error(
            "local storage unavailable: not cross-origin isolated"
        ));
        assert!(!is_fatal_open_error("open failed: nope"));
    }
}
