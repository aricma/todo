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
}
