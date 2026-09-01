//! CSV: the OPS flat serialization — `id,title,status,notes,metadata,
//! subtasks` (required fields as columns, everything else in the `metadata`
//! JSON, child-id lists in `subtasks`). Import is header-driven: `title`
//! and `status` are required, the `metadata` column merges as JSON, and
//! any additional column becomes a metadata entry. Quoted RFC 4180-ish.

use serde_json::Value;

use crate::model::Task;

use super::{task_to_ops, valid_metadata_key, OpsTask};

/// CSV: the OPS flat dialect — `id,title,status,notes,metadata,subtasks`
/// (the spec's required columns as columns, everything else in the
/// `metadata` JSON; child-id lists in `subtasks`). Quoted RFC 4180-ish
/// (commas/quotes/newlines).
pub fn serialize_csv(tasks: &[Task]) -> String {
    fn field(s: &str) -> String {
        if s.contains(',') || s.contains('"') || s.contains('\n') {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_string()
        }
    }
    let mut out = String::from("id,title,status,notes,metadata,subtasks\n");
    for t in tasks {
        let ops = task_to_ops(t);
        out.push_str(&format!(
            "{},{},{},{},{},{}\n",
            field(&t.id),
            field(&t.title),
            field(t.status.as_str()),
            field(t.notes.as_deref().unwrap_or("")),
            field(&serde_json::to_string(&ops.metadata).expect("metadata json")),
            field(&serde_json::to_string(&ops.subtasks).expect("subtasks json")),
        ));
    }
    out
}

/// Split a CSV line into fields, honoring doubled quotes (RFC 4180-ish).
/// RFC-4180 field splitter: quoted fields, escaped quotes (`""`), commas
/// inside quotes. Enough for the exporter's dialect (serialize_csv quotes
/// `,`/`"`/`\n`).
/// ponytail: newlines inside quoted fields are NOT handled (split runs per
/// line) — a multi-line title would break round-trip. UI can't produce one
/// today; swap for csv-core if it ever can.
fn split_csv(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                out.push(std::mem::take(&mut cur));
            }
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out
}

/// Parse an OPS-flat CSV, header-driven: `title` and `status` columns are
/// required; `id`, `notes`, `metadata` (JSON object) and `subtasks` (JSON
/// id list) are the other known columns. ANY additional column becomes a
/// metadata entry (OPS rule 5: keys match `^[a-z0-9_]{3,}$`). The pre-OPS
/// app dialect (`id,title,status,due_local,priority,location,tags`) imports
/// through the extra-column path and maps to typed fields via ops_to_task.
pub fn parse_csv_doc(text: &str) -> Result<Vec<OpsTask>, String> {
    let mut lines = text.lines();
    let header = lines.next().ok_or("empty csv")?;
    let header_fields = split_csv(header);
    let cols: Vec<&str> = header_fields.iter().map(|s| s.trim()).collect();
    let col = |name: &str| cols.iter().position(|c| *c == name);
    let title_i = col("title").ok_or("csv header must have a \"title\" column")?;
    let status_i = col("status").ok_or("csv header must have a \"status\" column")?;
    let id_i = col("id");
    let notes_i = col("notes");
    let metadata_i = col("metadata");
    let subtasks_i = col("subtasks");
    let known = |i: usize| {
        id_i == Some(i)
            || title_i == i
            || status_i == i
            || notes_i == Some(i)
            || metadata_i == Some(i)
            || subtasks_i == Some(i)
    };
    let extra: Vec<(usize, &str)> = cols
        .iter()
        .enumerate()
        .filter(|(i, _)| !known(*i))
        .map(|(i, c)| (i, *c))
        .collect();
    for (_, name) in &extra {
        if !valid_metadata_key(name) {
            return Err(format!(
                "csv column \"{name}\" is not a valid metadata key (OPS: ^[a-z0-9_]{{3,}}$)"
            ));
        }
    }
    let mut out = Vec::new();
    for (i, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let fields = split_csv(line);
        let f = |j: usize| fields.get(j).map(|s| s.as_str()).unwrap_or("");
        let mut metadata = serde_json::Map::new();
        if let Some(mi) = metadata_i {
            if !f(mi).is_empty() {
                match serde_json::from_str::<Value>(f(mi)) {
                    Ok(Value::Object(m)) => metadata.extend(m),
                    Ok(_) => {
                        return Err(format!(
                            "csv line {}: metadata column must be a JSON object",
                            i + 2
                        ))
                    }
                    Err(e) => return Err(format!("csv line {}: metadata column: {e}", i + 2)),
                }
            }
        }
        for (j, name) in &extra {
            if !f(*j).is_empty() {
                metadata.insert((*name).to_string(), Value::String(f(*j).to_string()));
            }
        }
        let subtasks = match subtasks_i {
            Some(sj) if !f(sj).is_empty() => serde_json::from_str::<Vec<Value>>(f(sj))
                .map_err(|e| format!("csv line {}: subtasks column: {e}", i + 2))?,
            _ => Vec::new(),
        };
        out.push(OpsTask {
            version: None,
            id: id_i.map(f).filter(|s| !s.is_empty()).map(str::to_string),
            title: f(title_i).to_string(),
            status: f(status_i).to_string(),
            notes: notes_i.map(f).filter(|s| !s.is_empty()).map(str::to_string),
            subtasks,
            metadata,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::super::{fixture_task, ops_to_task};
    use super::*;

    #[test]
    fn csv_is_ops_shaped_and_escapes_commas() {
        let mut t = fixture_task();
        t.title = "Buy, milk \"now\"".into();
        let csv = serialize_csv(&[t]);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], "id,title,status,notes,metadata,subtasks");
        // Commas/quotes in fields are quoted with doubled quotes.
        assert!(lines[1].contains("\"Buy, milk \"\"now\"\"\""));
        // Round-trip: the metadata JSON (typed fields) and the empty
        // subtasks column parse back losslessly.
        let parsed = parse_csv_doc(&csv).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].metadata["due_local"], "2026-03-15");
        assert_eq!(parsed[0].metadata["priority"], "high");
        assert_eq!(parsed[0].metadata["tags"], serde_json::json!(["errands"]));
        assert!(parsed[0].subtasks.is_empty());
    }

    #[test]
    fn csv_roundtrips_through_ops() {
        let t = fixture_task();
        let csv = serialize_csv(&[t.clone()]);
        let parsed = parse_csv_doc(&csv).unwrap();
        assert_eq!(parsed.len(), 1);
        let back = ops_to_task(
            parsed.into_iter().next().unwrap(),
            "2026-03-13T10:00:00.000Z",
        );
        assert_eq!(back.title, t.title);
        assert_eq!(back.due_local, t.due_local);
        assert_eq!(back.priority, t.priority);
        assert_eq!(back.location, t.location);
        assert_eq!(back.tags, t.tags);
        assert_eq!(back.status, t.status);
    }

    #[test]
    fn csv_parses_quoted_commas() {
        let mut t = fixture_task();
        t.title = "Buy, milk \"now\"".into();
        let csv = serialize_csv(&[t]);
        let parsed = parse_csv_doc(&csv).unwrap();
        assert_eq!(parsed[0].title, "Buy, milk \"now\"");
    }

    #[test]
    fn csv_imports_extra_columns_as_metadata() {
        // Foreign CSVs: any column beyond the known ones is metadata.
        let csv = "id,title,status,estimate,assignee\nt1,ship it,open,3,ada\n";
        let parsed = parse_csv_doc(csv).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].metadata["estimate"], "3");
        assert_eq!(parsed[0].metadata["assignee"], "ada");
        let t = ops_to_task(parsed.into_iter().next().unwrap(), "now");
        assert_eq!(t.metadata["estimate"], "3");
    }

    #[test]
    fn csv_rejects_invalid_extra_column_names() {
        let csv = "id,title,status,Bad Key\nt1,x,open,v\n";
        assert!(parse_csv_doc(csv).unwrap_err().contains("metadata key"));
    }

    #[test]
    fn csv_legacy_seven_column_header_imports() {
        // The pre-OPS dialect imports through the extra-column path and
        // maps to typed fields.
        let csv = "id,title,status,due_local,priority,location,tags\nt1,Buy milk,open,2026-03-15,high,shop,errands\n";
        let parsed = parse_csv_doc(csv).unwrap();
        let t = ops_to_task(parsed.into_iter().next().unwrap(), "now");
        assert_eq!(t.due_local.as_deref(), Some("2026-03-15"));
        assert_eq!(t.priority.as_deref(), Some("high"));
        assert_eq!(t.location.as_deref(), Some("shop"));
        assert_eq!(t.tags, vec!["errands"]);
    }

    #[test]
    fn csv_roundtrips_metadata_and_subtask_lists() {
        // Opaque metadata + a child-id list survive a CSV round-trip: the
        // list rides in metadata through the MVP-flat app and lands back in
        // the subtasks column on export.
        let mut t = fixture_task();
        t.metadata = serde_json::json!({ "favorite_thing": "cats" });
        t.metadata["subtasks"] = serde_json::json!(["t1a"]);
        let csv = serialize_csv(&[t.clone()]);
        let parsed = parse_csv_doc(&csv).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].metadata["favorite_thing"], "cats");
        assert_eq!(parsed[0].subtasks, vec![Value::String("t1a".into())]);
        let back = ops_to_task(
            parsed.into_iter().next().unwrap(),
            "2026-03-13T10:00:00.000Z",
        );
        assert_eq!(back.metadata["favorite_thing"], "cats");
        assert_eq!(back.metadata["subtasks"][0], "t1a");
        // The exporter puts the id list back in the column — and it parses
        // back out again (quoted, since it contains quotes).
        let back_csv = serialize_csv(&[back]);
        let reparsed = parse_csv_doc(&back_csv).unwrap();
        assert_eq!(reparsed[0].subtasks, vec![Value::String("t1a".into())]);
    }
}
