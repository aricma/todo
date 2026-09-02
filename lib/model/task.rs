use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Open,
    Done,
}

impl Status {
    pub fn as_str(&self) -> &'static str {
        match self {
            Status::Open => "open",
            Status::Done => "done",
        }
    }

    pub fn parse(s: &str) -> Status {
        if s == "done" {
            Status::Done
        } else {
            Status::Open
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Freq {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WeekdayName {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

impl WeekdayName {
    pub fn time(self) -> time::Weekday {
        use time::Weekday::*;
        match self {
            WeekdayName::Mon => Monday,
            WeekdayName::Tue => Tuesday,
            WeekdayName::Wed => Wednesday,
            WeekdayName::Thu => Thursday,
            WeekdayName::Fri => Friday,
            WeekdayName::Sat => Saturday,
            WeekdayName::Sun => Sunday,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recurrence {
    pub freq: Freq,
    pub interval: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weekdays: Option<Vec<WeekdayName>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub status: Status,
    pub notes: Option<String>,
    pub parent_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
    pub due_date: Option<String>,
    pub due_local: Option<String>,
    pub priority: Option<String>,
    pub location: Option<String>,
    pub tags: Vec<String>,
    pub recurrence: Option<Recurrence>,
    pub metadata: serde_json::Value,
}

/// Reschedule: move this instance's due date (postpone). Touches only the
/// due fields — recurrence is never read or written, so the schedule
/// (freq/interval/weekdays) stays intact; only this instance moves
/// (ADR-0013 spawns the next occurrence from the new due date).
/// Mirrors apply_parsed's injected tz seam so it is host-testable; the app
/// passes js_time::to_utc_iso.
impl Task {
    pub fn reschedule(
        &mut self,
        due: time::Date,
        time: Option<(u8, u8)>,
        to_utc_iso: &impl Fn(time::Date, Option<(u8, u8)>) -> String,
    ) {
        self.due_local = Some(due.to_string());
        self.due_date = Some(to_utc_iso(due, time));
        match time {
            Some((h, m)) => {
                self.metadata["due_time"] = serde_json::json!(format!("{h:02}:{m:02}"));
            }
            None => {
                if let Some(o) = self.metadata.as_object_mut() {
                    o.remove("due_time");
                }
            }
        }
    }
}

/// Unique task id (OPS `id`). Browser: crypto.randomUUID (secure context —
/// OPFS already requires one). Native (tests): process-unique counter.
pub fn new_id() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.crypto().ok())
            .map(|c| c.random_uuid())
            .unwrap_or_else(|| format!("todo-{}", uuid_fallback()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        format!("t{}-{}", std::process::id(), uuid_fallback())
    }
}

fn uuid_fallback() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_roundtrip() {
        assert_eq!(Status::parse("done"), Status::Done);
        assert_eq!(Status::parse("open"), Status::Open);
        assert_eq!(Status::parse("whatever"), Status::Open);
        assert_eq!(Status::Done.as_str(), "done");
    }

    #[test]
    fn ids_are_unique() {
        let a = new_id();
        let b = new_id();
        assert_ne!(a, b);
        assert!(!a.is_empty());
    }

    #[test]
    fn recurrence_json_roundtrip() {
        let r = Recurrence {
            freq: Freq::Weekly,
            interval: 2,
            weekdays: Some(vec![WeekdayName::Mon, WeekdayName::Wed]),
        };
        let s = serde_json::to_string(&r).unwrap();
        assert_eq!(
            s,
            r#"{"freq":"weekly","interval":2,"weekdays":["mon","wed"]}"#
        );
        let back: Recurrence = serde_json::from_str(&s).unwrap();
        assert_eq!(back, r);
    }

    fn reschedule_stub_to_utc_iso(d: time::Date, tm: Option<(u8, u8)>) -> String {
        match tm {
            Some((h, m)) => format!("{d}T{h:02}:{m:02}:00.000Z"),
            None => format!("{d}T00:00:00.000Z"),
        }
    }

    fn reschedule_d(y: i32, m: time::Month, day: u8) -> time::Date {
        time::Date::from_calendar_date(y, m, day).unwrap()
    }

    /// A recurring task with a due date (the p1 scenario: re-date the instance,
    /// keep the schedule).
    fn reschedule_task() -> Task {
        Task {
            id: "x".into(),
            title: "Standup".into(),
            status: Status::Open,
            notes: None,
            parent_id: None,
            created_at: "2026-09-01T08:00:00.000Z".into(),
            updated_at: "2026-09-01T08:00:00.000Z".into(),
            completed_at: None,
            due_date: Some("2026-09-02T00:00:00.000Z".into()),
            due_local: Some("2026-09-02".into()),
            priority: None,
            location: None,
            tags: Vec::new(),
            recurrence: Some(Recurrence {
                freq: Freq::Weekly,
                interval: 1,
                weekdays: None,
            }),
            metadata: serde_json::json!({}),
        }
    }

    #[test]
    fn reschedule_sets_due_local_and_midnight_utc() {
        let mut t = reschedule_task();
        t.reschedule(
            reschedule_d(2026, time::Month::September, 15),
            None,
            &reschedule_stub_to_utc_iso,
        );
        assert_eq!(t.due_local.as_deref(), Some("2026-09-15"));
        assert_eq!(t.due_date.as_deref(), Some("2026-09-15T00:00:00.000Z"));
        assert!(t.metadata.get("due_time").is_none());
    }

    #[test]
    fn reschedule_with_time_writes_datetime_and_due_time() {
        let mut t = reschedule_task();
        t.reschedule(
            reschedule_d(2026, time::Month::September, 15),
            Some((14, 30)),
            &reschedule_stub_to_utc_iso,
        );
        assert_eq!(t.due_date.as_deref(), Some("2026-09-15T14:30:00.000Z"));
        assert_eq!(t.metadata["due_time"], "14:30");
    }

    #[test]
    fn reschedule_clears_stale_due_time() {
        let mut t = reschedule_task();
        t.metadata["due_time"] = serde_json::json!("09:00");
        t.reschedule(
            reschedule_d(2026, time::Month::September, 15),
            None,
            &reschedule_stub_to_utc_iso,
        );
        assert!(t.metadata.get("due_time").is_none());
    }

    #[test]
    fn reschedule_keeps_recurrence_schedule_intact() {
        // p1: re-date only the current instance — the schedule stays.
        let mut t = reschedule_task();
        let rec = t.recurrence.clone().unwrap();
        t.reschedule(
            reschedule_d(2026, time::Month::September, 20),
            None,
            &reschedule_stub_to_utc_iso,
        );
        assert_eq!(t.recurrence, Some(rec));
        assert_eq!(t.due_local.as_deref(), Some("2026-09-20"));
    }
}
