//! App state machine (MVP.md §4), pure and host-testable: the state model
//! plus the transition actions. The db round-trips stay in the app bin
//! (db::open_flow); this module turns their results into the next state, so
//! the machine is testable away from the UI (states-module inbox todo).

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Booting,
    SourceSelect {
        cached: Option<String>,
        warning: Option<String>,
    },
    Opening {
        path: String,
    },
    Ready {
        path: String,
    },
    Error(String),
}

impl AppState {
    /// Booting → Opening (cached source) or SourceSelect (fresh browser).
    pub fn boot(cached: Option<String>) -> AppState {
        match cached {
            Some(path) => AppState::Opening { path },
            None => AppState::SourceSelect {
                cached: None,
                warning: None,
            },
        }
    }

    /// Opening → Ready on success; Error for sqlite-init failures (fatal,
    /// nothing to retry); back to SourceSelect with a warning for
    /// source-open/migration failures (the source may have been wiped).
    pub fn opening_finished(self, result: Result<(), String>) -> AppState {
        let AppState::Opening { path } = self else {
            return self; // not in Opening — no transition
        };
        match result {
            Ok(()) => AppState::Ready { path },
            Err(e) if is_fatal_open_error(&e) => AppState::Error(e),
            Err(e) => AppState::SourceSelect {
                cached: Some(path),
                warning: Some(e),
            },
        }
    }
}

/// Which open failures are terminal. sqlite-wasm init is the only fatal one
/// — the worker is unusable, nothing to retry. Matches db::init's error
/// prefix (contract: db.rs keeps it stable).
pub fn is_fatal_open_error(e: &str) -> bool {
    e.starts_with("sqlite init failed")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot_with_cached_source_opens_it() {
        assert_eq!(
            AppState::boot(Some("work/todos.db".into())),
            AppState::Opening {
                path: "work/todos.db".into()
            }
        );
    }

    #[test]
    fn boot_without_cache_shows_source_select() {
        assert_eq!(
            AppState::boot(None),
            AppState::SourceSelect {
                cached: None,
                warning: None
            }
        );
    }

    #[test]
    fn opening_success_reaches_ready() {
        let s = AppState::Opening {
            path: "a/todos.db".into(),
        };
        assert_eq!(
            s.opening_finished(Ok(())),
            AppState::Ready {
                path: "a/todos.db".into()
            }
        );
    }

    #[test]
    fn opening_sqlite_init_failure_is_fatal() {
        let s = AppState::Opening {
            path: "a/todos.db".into(),
        };
        assert_eq!(
            s.opening_finished(Err("sqlite init failed: worker timeout".into())),
            AppState::Error("sqlite init failed: worker timeout".into())
        );
    }

    #[test]
    fn opening_source_failure_returns_to_select_with_warning() {
        let s = AppState::Opening {
            path: "a/todos.db".into(),
        };
        assert_eq!(
            s.opening_finished(Err("open failed: no such file".into())),
            AppState::SourceSelect {
                cached: Some("a/todos.db".into()),
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
    }

    #[test]
    fn fatal_error_classification() {
        assert!(is_fatal_open_error("sqlite init failed: boom"));
        assert!(!is_fatal_open_error("open failed: nope"));
    }
}
