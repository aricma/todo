//! Recurrence semantics (MVP.md §3.3, ADR-0006 + ADR-0013): schedule math
//! — `next_occurrence` (strictly after the current due; month/year overflow
//! clamps; weekday sets step to the next day in the set) — and the instance
//! chain (ADR-0013): completing an occurrence marks it Done and spawns a
//! NEW open instance linked by `metadata.recurring_id` + `instance_seq`, so
//! exports show the chain. `start_from_recurrence` seeds the first date of
//! a recurrence that has no due token (MVP.md §6), shared with the parser.

use time::{Date, Duration, Weekday};

use crate::model::{new_id, Freq, Recurrence, Status, Task};
use crate::parse::parse_iso_date;

pub fn add_months_clamped(d: Date, months: u32) -> Option<Date> {
    let total = d.year() * 12 + d.month() as u8 as i32 - 1 + months as i32;
    let y = total.div_euclid(12);
    let m = time::Month::try_from((total.rem_euclid(12) + 1) as u8).ok()?;
    let day = d.day().min(m.length(y));
    Date::from_calendar_date(y, m, day).ok()
}

pub fn next_weekday(after: Date, set: &[Weekday]) -> Option<Date> {
    (1..=7)
        .map(|n| after + Duration::days(n))
        .find(|c| set.contains(&c.weekday()))
}

/// Next occurrence strictly after `due`.
pub fn next_occurrence(due: Date, r: &Recurrence) -> Option<Date> {
    let interval = r.interval.max(1);
    match r.freq {
        Freq::Daily => Some(due + Duration::days(interval as i64)),
        Freq::Weekly => match &r.weekdays {
            Some(days) => next_weekday(due, &days.iter().map(|d| d.time()).collect::<Vec<_>>()),
            None => Some(due + Duration::days(7 * interval as i64)),
        },
        Freq::Monthly => add_months_clamped(due, interval),
        Freq::Yearly => add_months_clamped(due, 12 * interval),
    }
}

/// First occurrence from today when no explicit due token (MVP.md §6):
/// weekday sets → next day in the set after today, otherwise today + interval.
pub fn start_from_recurrence(today: Date, r: &Recurrence) -> Option<Date> {
    let interval = r.interval.max(1);
    match r.freq {
        Freq::Daily => Some(today + Duration::days(interval as i64)),
        Freq::Weekly => match &r.weekdays {
            Some(days) => next_weekday(today, &days.iter().map(|d| d.time()).collect::<Vec<_>>()),
            None => Some(today + Duration::days(7 * interval as i64)),
        },
        Freq::Monthly => add_months_clamped(today, interval),
        Freq::Yearly => add_months_clamped(today, 12 * interval),
    }
}

// ── instance chains (ADR-0013) ──────────────────────────────────────────
// Completing a recurring occurrence marks it Done and spawns a NEW open
// instance for the next occurrence. Instances are separate rows linked by
// metadata.recurring_id (stable chain uuid) + instance_seq (1-based order),
// so exports show the whole chain and analysis reads plain done rows. The
// id is minted lazily at the first recorded completion — rows created
// before this feature or imported from other tools have none — so a chain
// gets exactly one id, from its first recorded occurrence onward.

pub const RECURRING_ID: &str = "recurring_id";
pub const INSTANCE_SEQ: &str = "instance_seq";

pub fn recurring_id_of(t: &Task) -> Option<String> {
    t.metadata
        .get(RECURRING_ID)
        .and_then(|v| v.as_str())
        .map(String::from)
}

pub fn instance_seq_of(t: &Task) -> u32 {
    t.metadata
        .get(INSTANCE_SEQ)
        .and_then(|v| v.as_u64())
        .map(|n| n as u32)
        .unwrap_or(0)
}

pub fn stamp(t: &mut Task, id: &str, seq: u32) {
    if let Some(o) = t.metadata.as_object_mut() {
        o.insert(RECURRING_ID.into(), serde_json::json!(id));
        o.insert(INSTANCE_SEQ.into(), serde_json::json!(seq));
    }
}

/// Complete an open recurring occurrence (ADR-0013): the ticked instance
/// becomes Done (completed_at recorded; created_at + content untouched) and
/// a brand-new Open instance is spawned for the next occurrence strictly
/// after the current due (no due → first start from `today`, MVP §6),
/// copying the ticked instance's content at spawn time — so later edits to
/// the open instance (title, recurrence, …) affect it and every future
/// spawn, never the done rows behind it. The chain id is minted here when
/// the row has none. Returns (done_instance, spawned_next); next is None
/// for a non-recurring task or when no next occurrence is computable —
/// both defensive, unreachable from the recurring-task caller contract.
pub fn complete_occurrence(
    task: &Task,
    completed_at: &str,
    today: Date,
    to_utc_iso: &impl Fn(Date, Option<(u8, u8)>) -> String,
) -> (Task, Option<Task>) {
    let Some(rec) = &task.recurrence else {
        let mut done = task.clone();
        done.status = Status::Done;
        done.completed_at = Some(completed_at.to_string());
        done.updated_at = completed_at.to_string();
        return (done, None);
    };
    let id = recurring_id_of(task).unwrap_or_else(new_id);
    let seq = instance_seq_of(task).max(1);
    let due_time = task
        .metadata
        .get("due_time")
        .and_then(|v| v.as_str())
        .and_then(|s| {
            let (h, m) = s.split_once(':')?;
            Some((h.parse().ok()?, m.parse().ok()?))
        });

    let mut done = task.clone();
    done.status = Status::Done;
    done.completed_at = Some(completed_at.to_string());
    done.updated_at = completed_at.to_string();
    stamp(&mut done, &id, seq);

    let next_date = match task.due_local.as_deref().and_then(parse_iso_date) {
        Some(d) => next_occurrence(d, rec),
        None => start_from_recurrence(today, rec),
    };
    // next_occurrence/start_from_recurrence are total over valid
    // recurrences; a None here is unreachable and ends the chain quietly.
    let Some(next_date) = next_date else {
        return (done, None);
    };

    let mut next = done.clone();
    next.id = new_id();
    next.status = Status::Open;
    next.created_at = completed_at.to_string();
    next.completed_at = None;
    next.due_local = Some(next_date.to_string());
    next.due_date = Some(to_utc_iso(next_date, due_time));
    stamp(&mut next, &id, seq + 1);
    (done, Some(next))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::WeekdayName;

    fn d(y: i32, m: time::Month, day: u8) -> Date {
        Date::from_calendar_date(y, m, day).unwrap()
    }

    #[test]
    fn daily_steps_by_interval() {
        let r = Recurrence {
            freq: Freq::Daily,
            interval: 2,
            weekdays: None,
        };
        assert_eq!(
            next_occurrence(d(2026, time::Month::March, 14), &r),
            Some(d(2026, time::Month::March, 16))
        );
    }

    #[test]
    fn monthly_clamps_to_month_end() {
        let r = Recurrence {
            freq: Freq::Monthly,
            interval: 1,
            weekdays: None,
        };
        assert_eq!(
            next_occurrence(d(2026, time::Month::January, 31), &r),
            Some(d(2026, time::Month::February, 28))
        );
    }

    #[test]
    fn leap_year_feb29_clamps() {
        let r = Recurrence {
            freq: Freq::Yearly,
            interval: 1,
            weekdays: None,
        };
        assert_eq!(
            next_occurrence(d(2024, time::Month::February, 29), &r),
            Some(d(2025, time::Month::February, 28))
        );
    }

    #[test]
    fn weekly_steps_by_interval() {
        let r = Recurrence {
            freq: Freq::Weekly,
            interval: 2,
            weekdays: None,
        };
        assert_eq!(
            next_occurrence(d(2026, time::Month::March, 14), &r),
            Some(d(2026, time::Month::March, 28))
        );
    }

    #[test]
    fn weekday_set_steps_to_next_in_set() {
        let r = Recurrence {
            freq: Freq::Weekly,
            interval: 1,
            weekdays: Some(vec![WeekdayName::Mon, WeekdayName::Wed]),
        };
        // 2026-03-13 is a Friday → next set day is Monday 2026-03-16.
        assert_eq!(
            next_occurrence(d(2026, time::Month::March, 13), &r),
            Some(d(2026, time::Month::March, 16))
        );
    }

    #[test]
    fn strictly_after_never_same_day() {
        let r = Recurrence {
            freq: Freq::Weekly,
            interval: 1,
            weekdays: Some(vec![WeekdayName::Mon]),
        };
        // Due on Monday 2026-03-16 → next Monday, not the same day.
        assert_eq!(
            next_occurrence(d(2026, time::Month::March, 16), &r),
            Some(d(2026, time::Month::March, 23))
        );
    }

    #[test]
    fn month_end_rolls_into_next_month() {
        let r = Recurrence {
            freq: Freq::Monthly,
            interval: 1,
            weekdays: None,
        };
        assert_eq!(
            next_occurrence(d(2026, time::Month::December, 31), &r),
            Some(d(2027, time::Month::January, 31))
        );
    }

    // ── instance chains (ADR-0013) ────────────────────────────────────────

    fn stub_to_utc_iso(d: Date, tm: Option<(u8, u8)>) -> String {
        match tm {
            Some((h, m)) => format!("{d}T{h:02}:{m:02}:00.000Z"),
            None => format!("{d}T00:00:00.000Z"),
        }
    }

    /// Weekly recurring task, due Wed 2026-09-02, no chain id yet (legacy shape).
    fn chain_task() -> Task {
        Task {
            id: "inst-1".into(),
            title: "Standup".into(),
            status: Status::Open,
            notes: Some("camera on".into()),
            parent_id: None,
            created_at: "2026-08-01T08:00:00.000Z".into(),
            updated_at: "2026-09-01T08:00:00.000Z".into(),
            completed_at: None,
            due_date: Some("2026-09-02T09:00:00.000Z".into()),
            due_local: Some("2026-09-02".into()),
            priority: Some("high".into()),
            location: Some("office".into()),
            tags: vec!["work".into()],
            recurrence: Some(Recurrence {
                freq: Freq::Weekly,
                interval: 1,
                weekdays: None,
            }),
            metadata: serde_json::json!({ "due_time": "09:00" }),
        }
    }

    #[test]
    fn chain_helpers_read_metadata_defaults() {
        assert_eq!(recurring_id_of(&chain_task()), None);
        assert_eq!(instance_seq_of(&chain_task()), 0);
        let mut t = chain_task();
        stamp(&mut t, "chain-9", 4);
        assert_eq!(recurring_id_of(&t).as_deref(), Some("chain-9"));
        assert_eq!(instance_seq_of(&t), 4);
    }

    #[test]
    fn completing_mints_a_shared_id_and_marks_done_then_spawns_open_next() {
        let t = chain_task();
        let (done, next) = complete_occurrence(
            &t,
            "2026-09-02T09:05:00.000Z",
            d(2026, time::Month::September, 2),
            &stub_to_utc_iso,
        );
        let next = next.expect("recurring task spawns");
        // Done instance: status + completed_at recorded, content untouched.
        assert_eq!(done.status, Status::Done);
        assert_eq!(
            done.completed_at.as_deref(),
            Some("2026-09-02T09:05:00.000Z")
        );
        assert_eq!(done.due_local.as_deref(), Some("2026-09-02"));
        assert_eq!(done.title, "Standup");
        assert_eq!(done.created_at, "2026-08-01T08:00:00.000Z");
        assert_eq!(done.recurrence, t.recurrence);
        // Chain stamped on both, minted once, seq incremented.
        let id = recurring_id_of(&done).unwrap();
        assert!(!id.is_empty());
        assert_eq!(recurring_id_of(&done), recurring_id_of(&next));
        assert_eq!(instance_seq_of(&done), 1);
        assert_eq!(instance_seq_of(&next), 2);
        // Next instance: open, fresh row, next weekly occurrence +7, time kept.
        assert_eq!(next.status, Status::Open);
        assert_eq!(next.completed_at, None);
        assert_ne!(next.id, done.id);
        assert_eq!(next.created_at, done.completed_at.as_deref().unwrap());
        assert_eq!(next.due_local.as_deref(), Some("2026-09-09"));
        assert_eq!(next.due_date.as_deref(), Some("2026-09-09T09:00:00.000Z"));
        assert_eq!(next.title, "Standup");
        assert_eq!(next.notes, t.notes);
        assert_eq!(next.tags, t.tags);
        assert_eq!(next.location, t.location);
        assert_eq!(next.priority, t.priority);
        assert_eq!(next.recurrence, t.recurrence);
    }

    #[test]
    fn completing_preserves_an_existing_chain_id_and_seq() {
        let mut t = chain_task();
        stamp(&mut t, "chain-7", 4);
        let (done, next) = complete_occurrence(
            &t,
            "2026-09-09T09:05:00.000Z",
            d(2026, time::Month::September, 9),
            &stub_to_utc_iso,
        );
        assert_eq!(recurring_id_of(&done).as_deref(), Some("chain-7"));
        assert_eq!(instance_seq_of(&done), 4);
        let next = next.expect("recurring task spawns");
        assert_eq!(recurring_id_of(&next), recurring_id_of(&done));
        assert_eq!(instance_seq_of(&next), 5);
    }

    #[test]
    fn undated_recurring_instance_starts_from_today() {
        let mut t = chain_task();
        t.due_local = None;
        t.due_date = None;
        let (done, next) = complete_occurrence(
            &t,
            "2026-09-02T09:05:00.000Z",
            d(2026, time::Month::September, 2),
            &stub_to_utc_iso,
        );
        assert_eq!(done.status, Status::Done);
        // weekly → today + 7 (MVP §6 start rule).
        assert_eq!(next.unwrap().due_local.as_deref(), Some("2026-09-09"));
    }
    #[test]
    fn spawn_without_due_time_defaults_to_midnight() {
        let mut t = chain_task();
        if let Some(o) = t.metadata.as_object_mut() {
            o.remove("due_time");
        }
        let (done, next) = complete_occurrence(
            &t,
            "2026-09-02T09:05:00.000Z",
            d(2026, time::Month::September, 2),
            &stub_to_utc_iso,
        );
        assert_eq!(done.status, Status::Done);
        let next = next.expect("recurring task spawns");
        assert_eq!(next.due_date.as_deref(), Some("2026-09-09T00:00:00.000Z"));
    }
}
