//! Single-line quick-add parser (MVP.md §6, ADR-0005). Whitespace-separated
//! tokens; anything unrecognized joins the title verbatim. First due token
//! and first priority win.

use time::{Date, Duration};

use crate::model::{Freq, Recurrence, WeekdayName};
use crate::recurrence::{next_weekday, start_from_recurrence};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Parsed {
    pub title: String,
    pub tags: Vec<String>,
    pub location: Option<String>,
    pub priority: Option<String>,
    pub due: Option<Date>,
    /// Explicit due time ("at HH:MM"), first wins.
    pub time: Option<(u8, u8)>,
    pub recurrence: Option<Recurrence>,
}

const LONG_TO_SHORT: &[(&str, &str)] = &[
    ("monday", "mon"),
    ("tuesday", "tue"),
    ("wednesday", "wed"),
    ("thursday", "thu"),
    ("friday", "fri"),
    ("saturday", "sat"),
    ("sunday", "sun"),
];

fn weekday_name(s: &str) -> Option<WeekdayName> {
    let low = s.to_ascii_lowercase();
    let short = LONG_TO_SHORT
        .iter()
        .find(|(l, _)| *l == low)
        .map(|(_, s)| *s)
        .unwrap_or(&low);
    match short {
        "mon" => Some(WeekdayName::Mon),
        "tue" => Some(WeekdayName::Tue),
        "wed" => Some(WeekdayName::Wed),
        "thu" => Some(WeekdayName::Thu),
        "fri" => Some(WeekdayName::Fri),
        "sat" => Some(WeekdayName::Sat),
        "sun" => Some(WeekdayName::Sun),
        _ => None,
    }
}

fn sanitize(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect()
}

fn parse_priority(s: &str) -> Option<&'static str> {
    match s.to_ascii_lowercase().as_str() {
        "high" | "h" | "1" => Some("high"),
        "medium" | "m" | "2" => Some("medium"),
        "low" | "l" | "3" => Some("low"),
        _ => None,
    }
}

fn unit_freq(s: &str) -> Option<Freq> {
    match s {
        "day" | "days" => Some(Freq::Daily),
        "week" | "weeks" => Some(Freq::Weekly),
        "month" | "months" => Some(Freq::Monthly),
        "year" | "years" => Some(Freq::Yearly),
        _ => None,
    }
}

/// Try to consume an `every …` phrase. Returns (recurrence, tokens consumed
/// after "every").
fn parse_every(toks: &[&str]) -> Option<(Recurrence, usize)> {
    let first = toks.first()?.to_ascii_lowercase();
    if let Some(f) = unit_freq(&first) {
        return Some((
            Recurrence {
                freq: f,
                interval: 1,
                weekdays: None,
            },
            1,
        ));
    }
    if let Ok(n) = first.parse::<u32>() {
        if n >= 1 {
            if let Some(unit) = toks.get(1).and_then(|s| unit_freq(&s.to_ascii_lowercase())) {
                return Some((
                    Recurrence {
                        freq: unit,
                        interval: n,
                        weekdays: None,
                    },
                    2,
                ));
            }
        }
    }
    let names: Vec<WeekdayName> = first.split(',').filter_map(weekday_name).collect();
    if !names.is_empty() && first.split(',').count() == names.len() {
        return Some((
            Recurrence {
                freq: Freq::Weekly,
                interval: 1,
                weekdays: Some(names),
            },
            1,
        ));
    }
    None
}

fn push_title(out: &mut Parsed, t: &str) {
    if !out.title.is_empty() {
        out.title.push(' ');
    }
    out.title.push_str(t);
}

/// Reconstruct the editable single line from a task (MVP.md §5.1: clicking a
/// title re-opens the full original text): title + tags + location +
/// priority + recurrence + ISO due date.
pub fn task_to_line(t: &crate::model::Task) -> String {
    let mut parts = vec![t.title.clone()];
    for tag in &t.tags {
        parts.push(format!("#{tag}"));
    }
    if let Some(l) = &t.location {
        parts.push(format!("@{l}"));
    }
    if let Some(p) = &t.priority {
        parts.push(format!("!{p}"));
    }
    if let Some(r) = &t.recurrence {
        let unit = match r.freq {
            Freq::Daily => "day",
            Freq::Weekly => "week",
            Freq::Monthly => "month",
            Freq::Yearly => "year",
        };
        let tokens = match &r.weekdays {
            Some(days) => days
                .iter()
                .map(|d| d.time())
                .map(weekday_short)
                .collect::<Vec<_>>()
                .join(","),
            None if r.interval == 1 => unit.to_string(),
            None => format!("{} {unit}s", r.interval),
        };
        parts.push(format!("every {tokens}"));
    }
    if let Some(tm) = t.metadata.get("due_time").and_then(|v| v.as_str()) {
        parts.push(format!("at {tm}"));
    }
    if let Some(d) = &t.due_local {
        parts.push(d.clone());
    }
    parts.join(" ")
}

// ── Parsed → Task (MVP.md §5.1: the string pipeline's second half) ────────

/// The one wasm-touching step in the Parsed → Task pipeline: a local
/// calendar date (+ optional explicit time) → UTC ISO instant. Injected so
/// everything else is host-testable; the app passes `js_time::to_utc_iso`
/// (js_sys date math), tests pass a stub.
pub fn apply_parsed(
    t: &mut crate::model::Task,
    p: &Parsed,
    now: &str,
    to_utc_iso: &impl Fn(time::Date, Option<(u8, u8)>) -> String,
) {
    t.title = p.title.clone();
    t.tags = p.tags.clone();
    t.location = p.location.clone();
    t.priority = p.priority.clone();
    t.recurrence = p.recurrence.clone();
    t.due_local = p.due.map(|d| d.to_string());
    // due_date carries the explicit time when one was picked; otherwise the
    // local-midnight sentinel. metadata.due_time mirrors the explicit time so
    // "no time" and "00:00" stay distinguishable and the line round-trips.
    t.due_date = match (p.due, p.time) {
        (Some(d), Some((h, m))) => Some(to_utc_iso(d, Some((h, m)))),
        (Some(d), None) => Some(to_utc_iso(d, None)),
        _ => None,
    };
    match p.time {
        Some((h, m)) => {
            t.metadata["due_time"] = serde_json::json!(format!("{h:02}:{m:02}"));
        }
        None => {
            if let Some(o) = t.metadata.as_object_mut() {
                o.remove("due_time");
            }
        }
    }
    t.updated_at = now.to_string();
}

/// Build a fresh task from a parsed line (quick-add submit).
pub fn task_from_parsed(
    p: &Parsed,
    now: &str,
    to_utc_iso: &impl Fn(time::Date, Option<(u8, u8)>) -> String,
) -> crate::model::Task {
    let mut t = crate::model::Task {
        id: crate::model::new_id(),
        title: String::new(),
        status: crate::model::Status::Open,
        notes: None,
        parent_id: None,
        created_at: now.to_string(),
        updated_at: now.to_string(),
        completed_at: None,
        due_date: None,
        due_local: None,
        priority: None,
        location: None,
        tags: Vec::new(),
        recurrence: None,
        metadata: serde_json::json!({}),
    };
    apply_parsed(&mut t, p, now, to_utc_iso);
    t
}

/// Relative due-date chip label (MVP.md §5.5): today/tomorrow/weekday
/// within a week, else `Mon Mar 14` (weekday included so far-out dates
/// are readable in Upcoming).
pub fn due_label(due_local: &str, today: Date) -> String {
    let Some(d) = parse_iso_date(due_local) else {
        return due_local.to_string();
    };
    if d == today {
        return "today".into();
    }
    if d == today + Duration::days(1) {
        return "tomorrow".into();
    }
    let diff_days = (d - today).whole_days();
    if (0..=7).contains(&diff_days) {
        return weekday_short(d.weekday()).to_string();
    }
    format!(
        "{} {} {}",
        weekday_short(d.weekday()),
        month_short(d.month()),
        d.day()
    )
}

/// Heading for an Upcoming-view day group: title case (like the Today
/// view's "Today (N)"), with the weekday always present — a raw ISO date is
/// not the best reference at a glance (inbox todo). Near dates get the full
/// "Weekday Mon DD" too, not a bare weekday.
pub fn upcoming_heading(due_local: &str, today: Date) -> String {
    let Some(d) = parse_iso_date(due_local) else {
        return "No date".into();
    };
    if d == today {
        return "Today".into();
    }
    if d == today + Duration::days(1) {
        return "Tomorrow".into();
    }
    format!(
        "{} {} {}",
        weekday_short(d.weekday()),
        month_short(d.month()),
        d.day()
    )
}

/// Full "Weekday Mon DD" reference for a date (the Today/Upcoming section
/// metadata next to shorthand headers like "Today" / "Tomorrow").
pub fn full_date_label(due_local: &str) -> String {
    let Some(d) = parse_iso_date(due_local) else {
        return due_local.to_string();
    };
    format!(
        "{} {} {}",
        weekday_short(d.weekday()),
        month_short(d.month()),
        d.day()
    )
}

fn weekday_short(w: time::Weekday) -> &'static str {
    use time::Weekday::*;
    match w {
        Monday => "Mon",
        Tuesday => "Tue",
        Wednesday => "Wed",
        Thursday => "Thu",
        Friday => "Fri",
        Saturday => "Sat",
        Sunday => "Sun",
    }
}

fn month_short(m: time::Month) -> &'static str {
    use time::Month::*;
    match m {
        January => "Jan",
        February => "Feb",
        March => "Mar",
        April => "Apr",
        May => "May",
        June => "Jun",
        July => "Jul",
        August => "Aug",
        September => "Sep",
        October => "Oct",
        November => "Nov",
        December => "Dec",
    }
}

/// Parse `YYYY-MM-DD`; `Date::from_calendar_date` validates ranges.
pub fn parse_iso_date(s: &str) -> Option<Date> {
    let mut it = s.split('-');
    let y = it.next()?.parse::<i32>().ok()?;
    let m = it.next()?.parse::<u8>().ok()?;
    let d = it.next()?.parse::<u8>().ok()?;
    if it.next().is_some() {
        return None;
    }
    Date::from_calendar_date(y, time::Month::try_from(m).ok()?, d).ok()
}

/// Parse a `HH:MM` time (24h, hours 0-23, minutes 0-59).
pub fn parse_hhmm(t: &str) -> Option<(u8, u8)> {
    let (h, m) = t.split_once(':')?;
    let h = h.parse::<u8>().ok()?;
    let m = m.parse::<u8>().ok()?;
    (h <= 23 && m <= 59).then_some((h, m))
}

/// Byte offsets of every non-overlapping case-insensitive occurrence of
/// `needle` in `haystack` (inbox search highlighting). Empty needle → no
/// matches. Offsets are byte ranges into the original string (char-indexed,
/// so Unicode input stays sliceable).
pub fn match_spans(haystack: &str, needle: &str) -> Vec<(usize, usize)> {
    if needle.is_empty() {
        return vec![];
    }
    let hay_chars: Vec<(usize, char)> = haystack.char_indices().collect();
    let hay_lower: Vec<char> = haystack.to_lowercase().chars().collect();
    let needle_lower: Vec<char> = needle.to_lowercase().chars().collect();
    // A count-changing fold (e.g. İ U+0130 → two chars) desyncs hay_chars from
    // hay_lower — skip highlighting rather than risk a wrong offset or a panic.
    if hay_lower.len() != hay_chars.len() || needle_lower.len() > hay_lower.len() {
        return vec![];
    }
    let mut out = Vec::new();
    let mut i = 0;
    while i + needle_lower.len() <= hay_lower.len() {
        if hay_lower[i..i + needle_lower.len()] == needle_lower[..] {
            let (start_byte, _) = hay_chars[i];
            let (end_byte, last_char) = hay_chars[i + needle_lower.len() - 1];
            out.push((start_byte, end_byte + last_char.len_utf8()));
            i += needle_lower.len();
        } else {
            i += 1;
        }
    }
    out
}

pub fn parse(line: &str, today: Date) -> Parsed {
    let mut out = Parsed::default();
    let toks: Vec<&str> = line.split_whitespace().collect();
    let mut i = 0;
    while i < toks.len() {
        let t = toks[i];
        if t.starts_with('#') && t.len() > 1 {
            out.tags.push(sanitize(&t[1..]));
        } else if t.starts_with('@') && t.len() > 1 {
            if out.location.is_none() {
                out.location = Some(sanitize(&t[1..]));
            }
        } else if t.starts_with('!') && t.len() > 1 {
            if out.priority.is_none() {
                out.priority = parse_priority(&t[1..]).map(str::to_string);
            }
        } else if t.eq_ignore_ascii_case("every") {
            match parse_every(&toks[i + 1..]) {
                Some((r, used)) => {
                    out.recurrence = Some(r);
                    i += used;
                }
                None => push_title(&mut out, t),
            }
        } else if t.eq_ignore_ascii_case("next") {
            if let Some(w) = toks.get(i + 1).and_then(|s| weekday_name(s)) {
                if out.due.is_none() {
                    out.due = next_weekday(today, &[w.time()]);
                }
                i += 1;
            } else {
                push_title(&mut out, t);
            }
        } else if t.eq_ignore_ascii_case("in") {
            match (
                toks.get(i + 1).and_then(|s| s.parse::<i64>().ok()),
                toks.get(i + 2).map(|s| s.to_ascii_lowercase()),
            ) {
                (Some(n), Some(unit))
                    if n > 0 && matches!(unit.as_str(), "day" | "days" | "week" | "weeks") =>
                {
                    if out.due.is_none() {
                        let days = if unit.starts_with("week") { n * 7 } else { n };
                        out.due = Some(today + Duration::days(days));
                    }
                    i += 2;
                }
                _ => push_title(&mut out, t),
            }
        } else if t.eq_ignore_ascii_case("at") {
            if let Some((h, m)) = toks.get(i + 1).and_then(|s| parse_hhmm(s)) {
                if out.time.is_none() {
                    out.time = Some((h, m));
                }
                i += 1;
            } else {
                push_title(&mut out, t);
            }
        } else if t.eq_ignore_ascii_case("today") {
            if out.due.is_none() {
                out.due = Some(today);
            }
        } else if t.eq_ignore_ascii_case("tomorrow") || t.eq_ignore_ascii_case("tom") {
            if out.due.is_none() {
                out.due = Some(today + Duration::days(1));
            }
        } else if let Some(w) = weekday_name(t) {
            if out.due.is_none() {
                out.due = Some(if today.weekday() == w.time() {
                    today
                } else {
                    next_weekday(today, &[w.time()]).expect("a weekday within 7 days")
                });
            }
        } else if let Some(d) = parse_iso_date(t) {
            if out.due.is_none() {
                out.due = Some(d);
            }
        } else {
            push_title(&mut out, t);
        }
        i += 1;
    }
    out.title = out.title.trim().to_string();
    if out.due.is_none() {
        if let Some(r) = &out.recurrence {
            out.due = start_from_recurrence(today, r);
        }
    }
    out
}

/// Kind of a scraped token — which parser rule matched (quick-add
/// in-input highlighting).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Due,
    Time,
    Recurrence,
    Priority,
    Location,
    Tag,
}

/// A scraped token's position in the line: byte offsets into `line`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenSpan {
    pub start: usize,
    pub end: usize,
    pub kind: TokenKind,
}

/// Byte offsets of the whitespace-separated words in `line`.
fn word_spans(line: &str) -> Vec<(usize, usize, &str)> {
    let mut out = Vec::new();
    let mut rest = line;
    let mut base = 0usize;
    loop {
        let trimmed = rest.trim_start_matches(char::is_whitespace);
        base += rest.len() - trimmed.len();
        if trimmed.is_empty() {
            break;
        }
        let end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
        out.push((base, base + end, &trimmed[..end]));
        rest = &trimmed[end..];
        base += end;
    }
    out
}

/// Locate the scraped tokens in `line` as byte ranges, mirroring `parse`
/// (same recognition rules, first-wins). Multi-word phrases ("every 2
/// weeks", "next fri", "in 3 days") span all their words. The quick-add
/// input highlights these ranges in place; `parse` remains the authority
/// for what gets stored.
pub fn highlight_spans(line: &str) -> Vec<TokenSpan> {
    // Only completed words are classified: a trailing word with no space
    // after it is still being typed, so it stays plain until the user
    // presses space (mid-typing glitches otherwise — the highlight would
    // flicker as partial words briefly match, e.g. "tom").
    let mut words = word_spans(line);
    if !line.ends_with(char::is_whitespace) {
        words.pop();
    }
    let mut out = Vec::new();
    let mut due_seen = false;
    let mut prio_seen = false;
    let mut loc_seen = false;
    let mut i = 0;
    while i < words.len() {
        let (start, end, t) = words[i];
        let mut push_due = |out: &mut Vec<TokenSpan>, s: usize, e: usize| {
            if !due_seen {
                due_seen = true;
                out.push(TokenSpan {
                    start: s,
                    end: e,
                    kind: TokenKind::Due,
                });
            }
        };
        if t.starts_with('#') && t.len() > 1 {
            out.push(TokenSpan {
                start,
                end,
                kind: TokenKind::Tag,
            });
        } else if t.starts_with('@') && t.len() > 1 {
            if !loc_seen {
                loc_seen = true;
                out.push(TokenSpan {
                    start,
                    end,
                    kind: TokenKind::Location,
                });
            }
        } else if t.starts_with('!') && t.len() > 1 {
            if !prio_seen && parse_priority(&t[1..]).is_some() {
                prio_seen = true;
                out.push(TokenSpan {
                    start,
                    end,
                    kind: TokenKind::Priority,
                });
            }
        } else if t.eq_ignore_ascii_case("every") {
            if let Some((_, used)) =
                parse_every(&words[i + 1..].iter().map(|w| w.2).collect::<Vec<_>>())
            {
                let last_end = words[i + used].1;
                out.push(TokenSpan {
                    start,
                    end: last_end,
                    kind: TokenKind::Recurrence,
                });
                i += used;
            }
        } else if t.eq_ignore_ascii_case("next") {
            if words
                .get(i + 1)
                .is_some_and(|w| weekday_name(w.2).is_some())
            {
                let last_end = words[i + 1].1;
                push_due(&mut out, start, last_end);
                i += 1;
            }
        } else if t.eq_ignore_ascii_case("in") {
            if let (Some(n), Some(unit)) = (
                words.get(i + 1).and_then(|w| w.2.parse::<i64>().ok()),
                words.get(i + 2).map(|w| w.2.to_ascii_lowercase()),
            ) {
                if n > 0 && matches!(unit.as_str(), "day" | "days" | "week" | "weeks") {
                    let last_end = words[i + 2].1;
                    push_due(&mut out, start, last_end);
                    i += 2;
                }
            }
        } else if t.eq_ignore_ascii_case("at") {
            if let Some((_, _)) = words.get(i + 1).and_then(|w| parse_hhmm(w.2)) {
                let last_end = words[i + 1].1;
                out.push(TokenSpan {
                    start,
                    end: last_end,
                    kind: TokenKind::Time,
                });
                i += 1;
            }
        } else if t.eq_ignore_ascii_case("today")
            || t.eq_ignore_ascii_case("tomorrow")
            || t.eq_ignore_ascii_case("tom")
            || weekday_name(t).is_some()
            || parse_iso_date(t).is_some()
        {
            push_due(&mut out, start, end);
        }
        i += 1;
    }
    out
}

/// Autocomplete candidates for the trailing token: `#` → existing tags,
/// `@` → existing locations, `!` → the three priorities. Full tokens
/// (prefix included) so they can be inserted as-is; prefix match, capped
/// at 6.
pub fn suggest_for(line: &str, tags: Vec<String>, locs: Vec<String>) -> Vec<String> {
    let Some(tok) = line.split_whitespace().last() else {
        return Vec::new();
    };
    let (choices, rest, prefix): (Vec<String>, String, &str) =
        if let Some(r) = tok.strip_prefix('#') {
            (tags, r.to_ascii_lowercase(), "#")
        } else if let Some(r) = tok.strip_prefix('@') {
            (locs, r.to_ascii_lowercase(), "@")
        } else if let Some(r) = tok.strip_prefix('!') {
            (
                vec!["high".into(), "medium".into(), "low".into()],
                r.to_ascii_lowercase(),
                "!",
            )
        } else {
            return Vec::new();
        };
    choices
        .into_iter()
        .filter(|c| c.to_ascii_lowercase().starts_with(&rest))
        .take(6)
        .map(|c| format!("{prefix}{c}"))
        .collect()
}

/// Human recurrence chip label, shared by todo rows and the quick-add
/// preview (and used to match "remove this info" clicks).
pub fn recurrence_label(r: &Recurrence) -> String {
    let unit = match r.freq {
        Freq::Daily => "day",
        Freq::Weekly => "week",
        Freq::Monthly => "month",
        Freq::Yearly => "year",
    };
    let base = if r.interval == 1 {
        format!("every {unit}")
    } else {
        format!("every {} {unit}s", r.interval)
    };
    match &r.weekdays {
        Some(days) => {
            let days = days
                .iter()
                .map(|d| format!("{:?}", d).to_lowercase())
                .collect::<Vec<_>>()
                .join(",");
            format!("↻ {base} ({days})")
        }
        None => format!("↻ {base}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    fn spans_of(line: &str) -> Vec<(String, TokenKind)> {
        highlight_spans(line)
            .into_iter()
            .map(|s| (line[s.start..s.end].to_string(), s.kind))
            .collect()
    }

    fn today() -> Date {
        Date::from_calendar_date(2026, Month::March, 13).unwrap() // Friday
    }

    fn d(day: u8) -> Date {
        Date::from_calendar_date(2026, Month::March, day).unwrap()
    }

    #[test]
    fn extracts_tags_location_priority() {
        let p = parse("Buy milk #errands #home @shop !high", today());
        assert_eq!(p.title, "Buy milk");
        assert_eq!(p.tags, vec!["errands", "home"]);
        assert_eq!(p.location.as_deref(), Some("shop"));
        assert_eq!(p.priority.as_deref(), Some("high"));
    }

    #[test]
    fn priority_aliases() {
        assert_eq!(parse("x !1", today()).priority.as_deref(), Some("high"));
        assert_eq!(parse("x !3", today()).priority.as_deref(), Some("low"));
        assert_eq!(parse("x !bogus", today()).priority, None);
    }

    #[test]
    fn first_wins_for_duplicates() {
        let p = parse("x !high !low", today());
        assert_eq!(p.priority.as_deref(), Some("high"));
        let p = parse("x tomorrow fri", today());
        assert_eq!(p.due, Some(today() + Duration::days(1)));
    }

    #[test]
    fn tom_is_a_tomorrow_shorthand() {
        assert_eq!(
            parse("x tom", today()).due,
            Some(today() + Duration::days(1))
        );
        assert_eq!(
            parse("x TOM", today()).due,
            Some(today() + Duration::days(1))
        );
        // In-input highlighting treats it like tomorrow too.
        let spans = highlight_spans("x tom ");
        assert!(spans
            .iter()
            .any(|sp| { sp.kind == TokenKind::Due && &"x tom "[sp.start..sp.end] == "tom" }));
    }

    #[test]
    fn tomorrow_and_iso() {
        assert_eq!(
            parse("x tomorrow", today()).due,
            Some(today() + Duration::days(1))
        );
        let june = Date::from_calendar_date(2026, Month::June, 1).unwrap();
        assert_eq!(parse("x 2026-06-01", today()).due, Some(june));
    }

    #[test]
    fn due_label_far_dates_include_weekday() {
        // 2026-03-13 is a Friday; +10 days is Monday 2026-03-23.
        assert_eq!(due_label("2026-03-23", today()), "Mon Mar 23");
        // Within a week stays weekday-only.
        assert_eq!(due_label("2026-03-16", today()), "Mon");
        assert_eq!(due_label("2026-03-13", today()), "today");
        assert_eq!(due_label("2026-03-14", today()), "tomorrow");
    }

    #[test]
    fn upcoming_heading_title_case_with_weekday() {
        // 2026-08-30 is a Sunday (so 08-31 is Monday, 09-03 Thursday, 09-20 Sunday).
        let today = time::Date::from_calendar_date(2026, time::Month::August, 30).unwrap();
        assert_eq!(upcoming_heading("2026-08-30", today), "Today");
        assert_eq!(upcoming_heading("2026-08-31", today), "Tomorrow");
        // Near dates show the full human date too, not a bare weekday.
        assert_eq!(upcoming_heading("2026-09-03", today), "Thu Sep 3");
        assert_eq!(upcoming_heading("2026-09-20", today), "Sun Sep 20");
        assert_eq!(upcoming_heading("", today), "No date");
    }

    #[test]
    fn recurrence_label_formats() {
        let daily = Recurrence {
            freq: Freq::Daily,
            interval: 1,
            weekdays: None,
        };
        assert_eq!(recurrence_label(&daily), "↻ every day");
        let weekly2 = Recurrence {
            freq: Freq::Weekly,
            interval: 2,
            weekdays: Some(vec![WeekdayName::Mon, WeekdayName::Wed]),
        };
        assert_eq!(recurrence_label(&weekly2), "↻ every 2 weeks (mon,wed)");
    }

    #[test]
    fn weekday_inclusive_today_and_next() {
        assert_eq!(parse("x fri", today()).due, Some(today())); // today IS Friday
        assert_eq!(
            parse("x next fri", today()).due,
            Some(today() + Duration::days(7))
        );
        assert_eq!(parse("x mon", today()).due, Some(d(16))); // next Monday
    }

    #[test]
    fn in_n_days_and_weeks() {
        assert_eq!(
            parse("x in 3 days", today()).due,
            Some(today() + Duration::days(3))
        );
        assert_eq!(
            parse("x in 2 weeks", today()).due,
            Some(today() + Duration::days(14))
        );
        assert_eq!(parse("x in 0 days", today()).due, None);
    }

    #[test]
    fn recurrence_every_n_and_weekdays() {
        let p = parse("x every 2 weeks", today());
        assert_eq!(
            p.recurrence,
            Some(Recurrence {
                freq: Freq::Weekly,
                interval: 2,
                weekdays: None
            })
        );
        let p = parse("x every mon,wed", today());
        assert_eq!(
            p.recurrence.unwrap().weekdays,
            Some(vec![WeekdayName::Mon, WeekdayName::Wed])
        );
        assert_eq!(p.due, Some(d(16))); // next Monday after Friday
    }

    #[test]
    fn recurrence_without_date_starts_after_today() {
        assert_eq!(
            parse("x every day", today()).due,
            Some(today() + Duration::days(1))
        );
        assert_eq!(
            parse("x every week", today()).due,
            Some(today() + Duration::days(7))
        );
        assert_eq!(
            parse("x every 2 months", today()).due,
            Some(Date::from_calendar_date(2026, Month::May, 13).unwrap()) // Mar 13 + 2 months
        );
    }

    #[test]
    fn bare_markers_stay_literal() {
        let p = parse("why # now", today());
        assert_eq!(p.title, "why # now");
        assert!(p.tags.is_empty());
        let p = parse("every so often", today());
        assert_eq!(p.title, "every so often");
        assert!(p.recurrence.is_none());
        let p = parse("call me @ later", today());
        assert_eq!(p.title, "call me @ later");
    }

    #[test]
    fn unknown_words_become_title_verbatim() {
        let p = parse("  call   mom   ", today());
        assert_eq!(p.title, "call mom");
        let p = parse("! high!", today());
        assert_eq!(p.priority, None);
        assert_eq!(p.title, "! high!");
    }

    #[test]
    fn task_to_line_roundtrips_through_parse() {
        let p = parse(
            "Buy milk #errands @shop !high every 2 weeks tomorrow",
            today(),
        );
        let t = crate::model::Task {
            id: "x".into(),
            title: p.title,
            status: crate::model::Status::Open,
            notes: None,
            parent_id: None,
            created_at: "".into(),
            updated_at: "".into(),
            completed_at: None,
            due_date: None,
            due_local: p.due.map(|d| d.to_string()),
            priority: p.priority,
            location: p.location,
            tags: p.tags,
            recurrence: p.recurrence,
            metadata: serde_json::json!({}),
        };
        let line = task_to_line(&t);
        let reparsed = parse(&line, today());
        assert_eq!(reparsed.title, "Buy milk");
        assert_eq!(reparsed.tags, vec!["errands"]);
        assert_eq!(reparsed.location.as_deref(), Some("shop"));
        assert_eq!(reparsed.priority.as_deref(), Some("high"));
        assert_eq!(reparsed.recurrence, t.recurrence);
        assert_eq!(
            reparsed.due,
            t.due_local.as_deref().and_then(parse_iso_date)
        );
    }

    #[test]
    fn full_date_label_shows_weekday_month_day() {
        assert_eq!(full_date_label("2026-08-31"), "Mon Aug 31");
        assert_eq!(full_date_label("2026-03-14"), "Sat Mar 14");
        assert_eq!(full_date_label("nope"), "nope");
    }

    #[test]
    fn due_label_is_relative() {
        let t = today(); // 2026-03-13, Friday
        assert_eq!(due_label("2026-03-13", t), "today");
        assert_eq!(due_label("2026-03-14", t), "tomorrow");
        assert_eq!(due_label("2026-03-16", t), "Mon"); // within a week
        assert_eq!(due_label("2026-06-01", t), "Mon Jun 1"); // far out: weekday + date
        assert_eq!(due_label("2026-03-10", t), "Tue Mar 10"); // overdue
    }

    #[test]
    fn at_time_token_parses() {
        assert_eq!(parse("x at 14:30", today()).time, Some((14, 30)));
        assert_eq!(parse("x at 9:05 tomorrow", today()).time, Some((9, 5)));
        // Invalid times stay in the title.
        assert_eq!(parse("x at 25:00", today()).time, None);
        assert_eq!(parse("x at 14", today()).time, None);
        // First wins, like the other tokens.
        assert_eq!(parse("x at 08:00 at 20:00", today()).time, Some((8, 0)));
        // A bare "at" with no time stays literal.
        assert_eq!(parse("meet at noon", today()).title, "meet at noon");
    }

    #[test]
    fn at_time_highlighted_in_line() {
        let line = "x at 14:30 tomorrow";
        let spans = highlight_spans(line);
        assert!(spans
            .iter()
            .any(|sp| { sp.kind == TokenKind::Time && &line[sp.start..sp.end] == "at 14:30" }));
    }

    #[test]
    fn task_to_line_roundtrips_time() {
        let p = parse("Ship it at 14:30 every 2 weeks tomorrow", today());
        let t = crate::model::Task {
            id: "x".into(),
            title: p.title,
            status: crate::model::Status::Open,
            notes: None,
            parent_id: None,
            created_at: "".into(),
            updated_at: "".into(),
            completed_at: None,
            due_date: None,
            due_local: p.due.map(|d| d.to_string()),
            priority: p.priority,
            location: p.location,
            tags: p.tags,
            recurrence: p.recurrence,
            metadata: serde_json::json!({ "due_time": "14:30" }),
        };
        let line = task_to_line(&t);
        assert!(line.contains("at 14:30"), "line: {line}");
        let reparsed = parse(&line, today());
        assert_eq!(reparsed.time, Some((14, 30)));
        assert_eq!(reparsed.due, p.due);
    }

    #[test]
    fn last_uncompleted_word_is_not_highlighted() {
        // A word is only classified once the space after it is typed — the
        // trailing word is still being typed, so it must not highlight
        // (mid-typing glitches).
        assert!(highlight_spans("buy milk tomorrow").is_empty());
        assert!(highlight_spans("buy milk #errands").is_empty());
        assert!(highlight_spans("buy milk !high").is_empty());
        // With a trailing space the word is complete and gets highlighted.
        let spans = highlight_spans("buy milk tomorrow ");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].kind, TokenKind::Due);
        assert_eq!(
            &"buy milk tomorrow "[spans[0].start..spans[0].end],
            "tomorrow"
        );
    }

    #[test]
    fn multi_word_phrase_waits_for_its_last_word() {
        assert!(highlight_spans("x every 2 weeks").is_empty());
        assert!(highlight_spans("x next fri").is_empty());
        assert!(highlight_spans("x in 3 days").is_empty());
        let spans = highlight_spans("x every 2 weeks ");
        assert_eq!(spans.len(), 1);
        assert_eq!(
            &"x every 2 weeks "[spans[0].start..spans[0].end],
            "every 2 weeks"
        );
    }

    #[test]
    fn completed_words_before_the_last_are_highlighted() {
        // Completed words (space after them) highlight even while the last
        // word is still being typed.
        let spans = highlight_spans("buy milk #errands tomorrow");
        let tag = spans
            .iter()
            .find(|s| s.kind == TokenKind::Tag)
            .expect("tag");
        assert_eq!(
            &"buy milk #errands tomorrow"[tag.start..tag.end],
            "#errands"
        );
        // "tomorrow" is the trailing word — not yet highlighted.
        assert!(!spans.iter().any(|s| s.kind == TokenKind::Due));
        // Once a space follows it, the due token appears too.
        let spans = highlight_spans("buy milk #errands tomorrow ");
        assert!(spans.iter().any(|s| s.kind == TokenKind::Due));
    }

    #[test]
    fn spans_cover_each_scraped_token() {
        let spans = spans_of("Buy milk #errands @shop !high tomorrow ");
        assert_eq!(
            spans,
            vec![
                ("#errands".into(), TokenKind::Tag),
                ("@shop".into(), TokenKind::Location),
                ("!high".into(), TokenKind::Priority),
                ("tomorrow".into(), TokenKind::Due),
            ]
        );
    }

    #[test]
    fn plain_title_words_have_no_span() {
        assert_eq!(spans_of("call mom"), Vec::<(String, TokenKind)>::new());
        assert_eq!(spans_of("why # now"), Vec::<(String, TokenKind)>::new());
    }

    #[test]
    fn multi_word_phrases_span_all_words() {
        let spans = highlight_spans("x every 2 weeks ");
        assert_eq!(spans.len(), 1);
        assert_eq!(
            &"x every 2 weeks"[spans[0].start..spans[0].end],
            "every 2 weeks"
        );
        assert_eq!(spans[0].kind, TokenKind::Recurrence);
        let spans = highlight_spans("x next fri ");
        assert_eq!(&"x next fri"[spans[0].start..spans[0].end], "next fri");
        let spans = highlight_spans("x in 3 days ");
        assert_eq!(&"x in 3 days"[spans[0].start..spans[0].end], "in 3 days");
    }

    #[test]
    fn first_wins_matches_parse() {
        let spans = spans_of("x !high !low tomorrow fri ");
        assert_eq!(spans[0], ("!high".into(), TokenKind::Priority));
        assert_eq!(spans[1], ("tomorrow".into(), TokenKind::Due));
        assert_eq!(spans.len(), 2);
    }

    #[test]
    fn spans_agree_with_parse_values() {
        // Every span's substring, parsed alone, matches the parsed task bits.
        let line = "Ship it #ops @office !medium every 2 weeks 2026-09-01 ";
        let p = parse(line, today());
        let spans = highlight_spans(line);
        let text = |s: &TokenSpan| line[s.start..s.end].to_string();
        assert!(spans
            .iter()
            .any(|s| s.kind == TokenKind::Due && text(s) == "2026-09-01"));
        assert!(spans
            .iter()
            .any(|s| s.kind == TokenKind::Recurrence && text(s) == "every 2 weeks"));
        assert!(spans
            .iter()
            .any(|s| s.kind == TokenKind::Priority && text(s) == "!medium"));
        assert!(spans
            .iter()
            .any(|s| s.kind == TokenKind::Location && text(s) == "@office"));
        assert!(spans
            .iter()
            .any(|s| s.kind == TokenKind::Tag && text(s) == "#ops"));
        assert_eq!(p.due.map(|d| d.to_string()).as_deref(), Some("2026-09-01"));
    }

    /// Deterministic UTC-ISO converter for host tests (the app injects
    /// js_time::to_utc_iso; the real one is js_sys date math).
    fn stub_to_utc_iso(d: Date, tm: Option<(u8, u8)>) -> String {
        match tm {
            Some((h, m)) => format!("{d}T{h:02}:{m:02}:00.000Z"),
            None => format!("{d}T00:00:00.000Z"),
        }
    }

    fn base_task() -> crate::model::Task {
        crate::model::Task {
            id: "x".into(),
            title: "old".into(),
            status: crate::model::Status::Open,
            notes: None,
            parent_id: None,
            created_at: "2026-03-13T08:00:00.000Z".into(),
            updated_at: "2026-03-13T08:00:00.000Z".into(),
            completed_at: None,
            due_date: None,
            due_local: None,
            priority: None,
            location: None,
            tags: Vec::new(),
            recurrence: None,
            metadata: serde_json::json!({ "keep": 1 }),
        }
    }

    #[test]
    fn apply_parsed_replaces_task_fields() {
        let p = parse("Buy milk #errands @shop !high tomorrow", today());
        let mut t = base_task();
        apply_parsed(&mut t, &p, "2026-03-13T10:00:00.000Z", &stub_to_utc_iso);
        assert_eq!(t.title, "Buy milk");
        assert_eq!(t.tags, vec!["errands"]);
        assert_eq!(t.location.as_deref(), Some("shop"));
        assert_eq!(t.priority.as_deref(), Some("high"));
        assert_eq!(t.due_local.as_deref(), Some("2026-03-14"));
        assert_eq!(t.due_date.as_deref(), Some("2026-03-14T00:00:00.000Z"));
        assert_eq!(t.updated_at, "2026-03-13T10:00:00.000Z");
        // Unknown metadata rides along (OPS escape hatch).
        assert_eq!(t.metadata["keep"], 1);
    }

    #[test]
    fn apply_parsed_writes_and_clears_due_time() {
        let p = parse("Ship it at 14:30 tomorrow", today());
        let mut t = base_task();
        apply_parsed(&mut t, &p, "now", &stub_to_utc_iso);
        assert_eq!(t.due_date.as_deref(), Some("2026-03-14T14:30:00.000Z"));
        assert_eq!(t.metadata["due_time"], "14:30");

        // Removing the time token clears the metadata key and the sentinel.
        let p = parse("Ship it tomorrow", today());
        apply_parsed(&mut t, &p, "now", &stub_to_utc_iso);
        assert!(t.metadata.get("due_time").is_none());
        assert_eq!(t.due_date.as_deref(), Some("2026-03-14T00:00:00.000Z"));
    }

    #[test]
    fn apply_parsed_without_due_clears_due() {
        let p = parse("Just a title", today());
        let mut t = base_task();
        t.due_local = Some("2026-03-14".into());
        t.due_date = Some("2026-03-14T00:00:00.000Z".into());
        t.metadata["due_time"] = serde_json::json!("14:30");
        apply_parsed(&mut t, &p, "now", &stub_to_utc_iso);
        assert_eq!(t.due_local, None);
        assert_eq!(t.due_date, None);
        assert!(t.metadata.get("due_time").is_none());
    }

    #[test]
    fn task_from_parsed_builds_fresh_task() {
        let p = parse("New task #work tomorrow", today());
        let t = task_from_parsed(&p, "2026-03-13T10:00:00.000Z", &stub_to_utc_iso);
        assert!(!t.id.is_empty());
        assert_eq!(t.status, crate::model::Status::Open);
        assert_eq!(t.title, "New task");
        assert_eq!(t.tags, vec!["work"]);
        assert_eq!(t.created_at, "2026-03-13T10:00:00.000Z");
        assert_eq!(t.updated_at, "2026-03-13T10:00:00.000Z");
        assert_eq!(t.completed_at, None);
        assert_eq!(t.due_local.as_deref(), Some("2026-03-14"));
    }

    #[test]
    fn match_spans_finds_case_insensitive_occurrences() {
        assert_eq!(
            match_spans("Buy MILK and milk", "milk"),
            vec![(4, 8), (13, 17)]
        );
    }

    #[test]
    fn match_spans_empty_needle_or_no_match_is_empty() {
        assert!(match_spans("anything", "").is_empty());
        assert!(match_spans("Buy milk", "cheese").is_empty());
    }

    #[test]
    fn match_spans_adjacent_occurrences_do_not_overlap() {
        // Non-overlapping: the second "aa" is skipped ("aaa").
        assert_eq!(match_spans("aaa", "aa"), vec![(0, 2)]);
    }

    #[test]
    fn match_spans_offsets_are_byte_ranges_on_char_boundaries() {
        // é is 2 UTF-8 bytes; the offsets slice the original string cleanly.
        assert_eq!(match_spans("Café café", "café"), vec![(0, 5), (6, 11)]);
        assert_eq!(&"Café café"[0..5], "Café");
    }

    #[test]
    fn match_spans_count_changing_fold_is_skipped() {
        // İ (U+0130) lowercases to two chars — a count-changing fold. Skipping
        // (empty result) beats a panic or a wrong byte offset.
        assert!(match_spans("İstanbul", "stanbul").is_empty());
        assert!(match_spans("İ", "İ").is_empty());
    }
}
