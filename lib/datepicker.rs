//! Smart datepicker logic (design ref: tmp/datepicker.html): quick-select
//! preset dates and calendar-grid dimensions. Pure and host-tested; the
//! browser component (app/components/smart_date_picker.rs) renders the
//! aricma-ui template and calls these.

use time::{Date, Duration, Month};

/// The quick-select presets offered by the picker. Dates are computed from
/// `today` — the same weekday arithmetic as the reference demo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    Today,
    Tomorrow,
    /// Thursday, or +2 days when today is Thursday or later.
    LaterThisWeek,
    /// The upcoming Saturday (next Saturday when today is Saturday).
    ThisWeekend,
    /// The next Monday (a week out when today is Monday).
    NextWeek,
    /// Next week's Saturday.
    NextWeekend,
    /// Postpone: the selected date (or today) + 7 days.
    Postpone,
}

/// Resolve a preset against `today` and the current `selected` date
/// (Postpone re-dates the selection; the others ignore it).
pub fn preset_date(preset: Preset, today: Date, selected: Option<Date>) -> Date {
    use Preset::*;
    // 0 = Monday .. 6 = Sunday (time crate), matching the demo's Mon-first
    // "later this week" / "next week" targets.
    let mon0 = today.weekday().number_days_from_monday();
    // 0 = Sunday .. 6 = Saturday (calendar column order).
    let sun0 = (mon0 + 1) % 7;
    match preset {
        Today => today,
        Tomorrow => today + Duration::days(1),
        LaterThisWeek => {
            if mon0 < 3 {
                today + Duration::days((3 - mon0) as i64)
            } else {
                today + Duration::days(2)
            }
        }
        ThisWeekend => {
            let until_sat = (6 - sun0 + 7) % 7;
            today + Duration::days(if until_sat == 0 { 7 } else { until_sat as i64 })
        }
        NextWeek => {
            let until_mon = (7 - mon0) % 7;
            today + Duration::days(if until_mon == 0 { 7 } else { until_mon as i64 })
        }
        NextWeekend => today + Duration::days(((6 - sun0 + 7) % 7 + 7) as i64),
        Postpone => selected.unwrap_or(today) + Duration::days(7),
    }
}

/// 0 = Sunday .. 6 = Saturday — the weekday index of the 1st of the month,
/// i.e. the number of empty leading cells in the calendar grid.
pub fn first_weekday_sunday(date: Date) -> u8 {
    (date.weekday().number_days_from_monday() + 1) % 7
}

/// Number of days in `month` (1–12) of `year`.
pub fn days_in_month(year: i32, month: u8) -> u8 {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    let first_next =
        Date::from_calendar_date(next_year, Month::try_from(next_month).unwrap(), 1).unwrap();
    (first_next - Duration::days(1)).day()
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    fn d(y: i32, m: u8, day: u8) -> Date {
        Date::from_calendar_date(y, Month::try_from(m).unwrap(), day).unwrap()
    }

    // Anchors: 2026-09-01 is a Tuesday (the app's "next mon" → 2026-09-07).
    const TUE: (i32, u8, u8) = (2026, 9, 1);

    #[test]
    fn today_and_tomorrow() {
        assert_eq!(
            preset_date(Preset::Today, d(TUE.0, TUE.1, TUE.2), None),
            d(2026, 9, 1)
        );
        assert_eq!(
            preset_date(Preset::Tomorrow, d(TUE.0, TUE.1, TUE.2), None),
            d(2026, 9, 2)
        );
    }

    #[test]
    fn later_this_week_targets_thursday() {
        // Tuesday → Thursday; Thursday → Saturday (demo's +2 rule).
        assert_eq!(
            preset_date(Preset::LaterThisWeek, d(2026, 9, 1), None),
            d(2026, 9, 3)
        );
        assert_eq!(
            preset_date(Preset::LaterThisWeek, d(2026, 9, 3), None),
            d(2026, 9, 5)
        );
        // Saturday → Monday.
        assert_eq!(
            preset_date(Preset::LaterThisWeek, d(2026, 9, 5), None),
            d(2026, 9, 7)
        );
    }

    #[test]
    fn this_weekend_is_the_upcoming_saturday() {
        assert_eq!(
            preset_date(Preset::ThisWeekend, d(2026, 9, 1), None),
            d(2026, 9, 5)
        );
        // Already Saturday → next Saturday.
        assert_eq!(
            preset_date(Preset::ThisWeekend, d(2026, 9, 5), None),
            d(2026, 9, 12)
        );
        assert_eq!(
            preset_date(Preset::ThisWeekend, d(2026, 9, 6), None),
            d(2026, 9, 12)
        );
    }

    #[test]
    fn next_week_is_next_monday() {
        assert_eq!(
            preset_date(Preset::NextWeek, d(2026, 9, 1), None),
            d(2026, 9, 7)
        );
        // Monday → a week out (never "today").
        assert_eq!(
            preset_date(Preset::NextWeek, d(2026, 9, 7), None),
            d(2026, 9, 14)
        );
        // Sunday → the next day.
        assert_eq!(
            preset_date(Preset::NextWeek, d(2026, 9, 13), None),
            d(2026, 9, 14)
        );
    }

    #[test]
    fn next_weekend_is_next_weeks_saturday() {
        assert_eq!(
            preset_date(Preset::NextWeekend, d(2026, 9, 1), None),
            d(2026, 9, 12)
        );
        assert_eq!(
            preset_date(Preset::NextWeekend, d(2026, 9, 4), None),
            d(2026, 9, 12)
        );
    }

    #[test]
    fn postpone_redates_the_selection_or_today() {
        assert_eq!(
            preset_date(Preset::Postpone, d(2026, 9, 1), Some(d(2026, 9, 5))),
            d(2026, 9, 12)
        );
        assert_eq!(
            preset_date(Preset::Postpone, d(2026, 9, 1), None),
            d(2026, 9, 8)
        );
    }

    #[test]
    fn grid_dims_feb_leap_and_month_ends() {
        assert_eq!(first_weekday_sunday(d(2026, 9, 1)), 2); // Tuesday → column 2
        assert_eq!(first_weekday_sunday(d(2026, 9, 7)), 1); // Monday → column 1
        assert_eq!(days_in_month(2026, 2), 28);
        assert_eq!(days_in_month(2024, 2), 29); // leap
        assert_eq!(days_in_month(2026, 12), 31);
        assert_eq!(days_in_month(2026, 9), 30);
    }
}
