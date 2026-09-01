//! Recurrence roll-forward (MVP.md §3.3, ADR-0006): the next occurrence is
//! strictly after the current due date; month/year overflow clamps to the
//! last day of the month; weekday sets step to the next day in the set.

use time::{Date, Duration, Weekday};

use crate::model::{Freq, Recurrence};

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
}
