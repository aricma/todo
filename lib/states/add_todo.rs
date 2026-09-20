//! Quick-add submission state (MVP.md §5.1): the parse → default-due → build
//! action, plus the error state for a host to show. The async db write stays in
//! the component (sqlite-wasm futures are !Send, ADR-0009) — `build` is the
//! host-testable action, `error` what the caller reports.

use time::Date;

use crate::model::Task;
use crate::parse::{parse, task_from_parsed};

/// The state a quick-add submission leaves behind.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AddTodoState {
    /// Persist error for the caller to show (capacity limit, db failure).
    pub error: Option<String>,
}

/// The quick-add action: parse the line, apply the day the caller is looking at
/// as the default due date when the line has none, build the Task. Persisting is the caller's
/// async db write; the result lands in [`AddTodoState`] via
/// [`AddTodoState::record_persist`].
pub fn build(
    line: &str,
    today: Date,
    default_due: Option<Date>,
    now: &str,
    to_utc_iso: &impl Fn(Date, Option<(u8, u8)>) -> String,
) -> Task {
    let mut parsed = parse(line, today);
    if parsed.due.is_none() {
        if let Some(d) = default_due {
            parsed.due = Some(d);
        }
    }
    task_from_parsed(&parsed, now, to_utc_iso)
}

impl AddTodoState {
    /// Fold an async persist result into the submission state.
    pub fn record_persist(&mut self, result: Result<(), String>) {
        self.error = result.err();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    fn stub_to_utc_iso(d: Date, tm: Option<(u8, u8)>) -> String {
        match tm {
            Some((h, m)) => format!("{d}T{h:02}:{m:02}:00.000Z"),
            None => format!("{d}T00:00:00.000Z"),
        }
    }

    fn today() -> Date {
        Date::from_calendar_date(2026, Month::March, 13).unwrap() // Friday
    }

    #[test]
    fn build_parses_the_line_into_a_task() {
        let t = build(
            "Buy milk #errands tomorrow",
            today(),
            None,
            "2026-03-13T10:00:00.000Z",
            &stub_to_utc_iso,
        );
        assert_eq!(t.title, "Buy milk");
        assert_eq!(t.tags, vec!["errands"]);
        assert_eq!(t.due_local.as_deref(), Some("2026-03-14"));
        assert_eq!(t.due_date.as_deref(), Some("2026-03-14T00:00:00.000Z"));
    }

    #[test]
    fn build_applies_the_view_default_due() {
        // Dated views (Today/Upcoming) default undated lines to their date.
        let t = build("Buy milk", today(), Some(today()), "now", &stub_to_utc_iso);
        assert_eq!(t.due_local.as_deref(), Some("2026-03-13"));
    }

    #[test]
    fn build_keeps_explicit_due_over_the_default() {
        let t = build(
            "Buy milk tomorrow",
            today(),
            Some(today()),
            "now",
            &stub_to_utc_iso,
        );
        assert_eq!(t.due_local.as_deref(), Some("2026-03-14"));
    }

    #[test]
    fn record_persist_holds_the_error_and_clears_on_success() {
        let mut s = AddTodoState::default();
        s.record_persist(Err("todo limit reached (1000)".into()));
        assert_eq!(s.error.as_deref(), Some("todo limit reached (1000)"));
        s.record_persist(Ok(()));
        assert_eq!(s.error, None);
    }
}
