//! Markdown: an OPS checklist — one `- [ ]/- [x]` line per task, its
//! `metadata` JSON in an HTML comment so round-trips are lossless. Lines
//! without a comment are legacy token lines (title + tags/location/
//! priority/recurrence/due) and still import via the app parser.

use serde_json::{Map, Value};

use crate::model::{Status, Task};

use super::{task_to_ops, OpsTask};

/// Markdown: an OPS checklist — one `- [ ]/- [x]` line per task, its
/// `metadata` JSON in an HTML comment (the spec's markdown dialect) so
/// round-trips are lossless. Legacy token lines still import
/// (parse_markdown_doc falls back to the app parser).
pub fn serialize_markdown(tasks: &[Task]) -> String {
    let mut out = String::new();
    for t in tasks {
        let check = if t.status == Status::Done { "x" } else { " " };
        let ops = task_to_ops(t);
        let meta = serde_json::to_string(&ops.metadata).expect("metadata json");
        out.push_str(&format!(
            "- [{check}] {} <!-- metadata: {meta} -->\n",
            t.title
        ));
    }
    out
}

/// Split `title <!-- metadata: {…} -->` into (title, json).
fn split_metadata_comment(line: &str) -> Option<(&str, &str)> {
    const OPEN: &str = "<!-- metadata: ";
    let i = line.find(OPEN)?;
    let title = line[..i].trim_end();
    let rest = &line[i + OPEN.len()..];
    let end = rest.find("-->")?;
    Some((title, rest[..end].trim()))
}

/// Parse a markdown export. An OPS line carries its `metadata` as an HTML
/// comment (`- [ ] title <!-- metadata: {…} -->`) and imports losslessly;
/// a line without one is a legacy token line (title + tags/location/
/// priority/recurrence/due), re-parsed with the app parser.
pub fn parse_markdown_doc(text: &str, today: time::Date) -> Result<Vec<OpsTask>, String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim_end();
        let Some(body) = line.strip_prefix("- [") else {
            continue;
        };
        let check = body.chars().next().unwrap_or(' ');
        let Some(rest) = body.get(1..).and_then(|r| r.strip_prefix("]")) else {
            continue;
        };
        let rest = rest.trim_start();
        let status = if check == 'x' { "done" } else { "open" };
        // OPS metadata comment (lossless); else the legacy token line.
        if let Some((title, meta_raw)) = split_metadata_comment(rest) {
            let metadata: Map<String, Value> = serde_json::from_str(meta_raw)
                .map_err(|e| format!("markdown metadata comment: {e}"))?;
            out.push(OpsTask {
                version: None,
                id: None,
                title: title.to_string(),
                status: status.into(),
                notes: None,
                subtasks: vec![],
                metadata,
            });
            continue;
        }
        let p = crate::parse::parse(rest, today);
        let mut metadata = serde_json::Map::new();
        if let Some(d) = p.due {
            metadata.insert("due_local".into(), serde_json::Value::String(d.to_string()));
        }
        if let Some(pr) = &p.priority {
            metadata.insert("priority".into(), serde_json::Value::String(pr.clone()));
        }
        if let Some(l) = &p.location {
            metadata.insert("location".into(), serde_json::Value::String(l.clone()));
        }
        if !p.tags.is_empty() {
            metadata.insert(
                "tags".into(),
                serde_json::Value::Array(
                    p.tags
                        .iter()
                        .map(|t| serde_json::Value::String(t.clone()))
                        .collect(),
                ),
            );
        }
        if let Some(r) = &p.recurrence {
            metadata.insert(
                "recurrence".into(),
                serde_json::to_value(r).expect("recurrence"),
            );
        }
        if let Some(tm) = p.time {
            metadata.insert(
                "due_time".into(),
                serde_json::Value::String(format!("{:02}:{:02}", tm.0, tm.1)),
            );
        }
        out.push(OpsTask {
            version: None,
            id: None,
            title: p.title,
            status: status.into(),
            notes: None,
            subtasks: Vec::new(),
            metadata,
        });
    }
    if out.is_empty() {
        return Err("markdown file has no checklist items".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::super::{fixture_task, ops_to_task};
    use super::*;
    use crate::model::Status;

    #[test]
    fn markdown_is_an_ops_checklist() {
        let out = serialize_markdown(&[fixture_task()]);
        let line = out.lines().next().unwrap();
        assert!(line.starts_with("- [ ] Buy milk"));
        // OPS metadata comment carries the app fields losslessly.
        assert!(line.contains("<!-- metadata: "));
        assert!(line.contains("\"due_local\":\"2026-03-15\""));
        assert!(line.contains("\"tags\":[\"errands\"]"));
        // Done tasks render checked.
        let mut t = fixture_task();
        t.status = Status::Done;
        assert!(serialize_markdown(&[t]).starts_with("- [x] "));
    }

    #[test]
    fn markdown_roundtrips_with_tokens() {
        let t = fixture_task();
        let today = time::Date::from_calendar_date(2026, time::Month::March, 13).unwrap();
        let out = serialize_markdown(&[t.clone()]);
        let parsed = parse_markdown_doc(&out, today).unwrap();
        assert_eq!(parsed.len(), 1);
        let back = ops_to_task(
            parsed.into_iter().next().unwrap(),
            "2026-03-13T10:00:00.000Z",
        );
        assert_eq!(back.title, t.title);
        assert_eq!(back.status, t.status);
        assert_eq!(back.due_local, t.due_local);
        assert_eq!(back.tags, t.tags);
    }

    #[test]
    fn markdown_roundtrips_metadata_comments() {
        let mut t = fixture_task();
        t.metadata = serde_json::json!({ "favorite_thing": "cats", "assignee": "ada" });
        let today = time::Date::from_calendar_date(2026, time::Month::March, 13).unwrap();
        let out = serialize_markdown(&[t.clone()]);
        assert!(out.contains("<!-- metadata: "));
        assert!(out.contains("favorite_thing"));
        let parsed = parse_markdown_doc(&out, today).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].title, t.title);
        assert_eq!(parsed[0].metadata["favorite_thing"], "cats");
        assert_eq!(parsed[0].metadata["due_local"], "2026-03-15");
        let back = ops_to_task(
            parsed.into_iter().next().unwrap(),
            "2026-03-13T10:00:00.000Z",
        );
        assert_eq!(back.metadata["favorite_thing"], "cats");
        assert_eq!(back.due_local, t.due_local);
        assert_eq!(back.tags, t.tags);
    }

    #[test]
    fn markdown_skips_title_and_blank_lines() {
        // The export carries the data source as an h1 page title.
        let today = time::Date::from_calendar_date(2026, time::Month::March, 13).unwrap();
        let parsed =
            parse_markdown_doc("# smoke-abc/todos.db\n\n- [ ] plan it #ops\n", today).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].title, "plan it");
        assert!(parsed[0].metadata.contains_key("tags"));
    }

    #[test]
    fn markdown_parses_done_and_open_lines() {
        let today = time::Date::from_calendar_date(2026, time::Month::March, 13).unwrap();
        let parsed = parse_markdown_doc("- [x] done task\n- [ ] open task #tag", today).unwrap();
        assert_eq!(parsed[0].status, "done");
        assert_eq!(parsed[0].title, "done task");
        assert_eq!(parsed[1].status, "open");
        assert_eq!(parsed[1].title, "open task");
        assert!(parsed[1].metadata.contains_key("tags"));
    }
}
