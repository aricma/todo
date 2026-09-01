//! JSON serializations: the tree-preserving OPS JSON doc (flat MVP tasks)
//! and the line-delimited JSONL variant. `validate_doc` enforces OPS rules
//! 1–5 for the tree doc; JSONL lines parse straight into `OpsTask`.

use serde_json::Value;

use crate::model::Task;

use super::{task_to_ops, valid_metadata_key, OpsDoc, OpsTask};

/// Validate a document against OPS rules 1–5 for flat MVP documents
/// (non-empty title, status in {open, done}, unique non-empty ids, empty
/// subtasks, metadata key charset). Returns all violations.
pub fn validate_doc(raw: &str) -> Result<OpsDoc, Vec<String>> {
    let mut errors = Vec::new();
    let doc: Value = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(e) => return Err(vec![format!("invalid JSON: {e}")]),
    };
    if !doc.get("tasks").is_some_and(|t| t.is_array()) {
        return Err(vec![
            "document must be an object with a \"tasks\" array".into()
        ]);
    }
    let doc: OpsDoc = match serde_json::from_value(doc) {
        Ok(d) => d,
        Err(e) => return Err(vec![format!("document shape: {e}")]),
    };
    let mut ids = std::collections::HashSet::new();
    for (i, t) in doc.tasks.iter().enumerate() {
        let tag = format!("task[{i}]");
        if t.title.trim().is_empty() {
            errors.push(format!("{tag}: missing non-empty title"));
        }
        if !matches!(t.status.as_str(), "open" | "done") {
            errors.push(format!("{tag}: status must be open|done"));
        }
        if let Some(id) = &t.id {
            if id.is_empty() {
                errors.push(format!("{tag}: id must be non-empty"));
            } else if !ids.insert(id.clone()) {
                errors.push(format!("{tag}: duplicate id {id}"));
            }
        }
        if !t.subtasks.is_empty() {
            errors.push(format!("{tag}: subtasks are not supported in this MVP"));
        }
        for k in t.metadata.keys() {
            if !valid_metadata_key(k) {
                errors.push(format!("{tag}: invalid metadata key \"{k}\""));
            }
        }
    }
    if errors.is_empty() {
        Ok(doc)
    } else {
        Err(errors)
    }
}

/// The tree-preserving OPS JSON export (all tasks → `{ "tasks": […] }`).
pub fn serialize_export(tasks: &[Task]) -> String {
    let doc = OpsDoc {
        tasks: tasks.iter().map(task_to_ops).collect(),
    };
    serde_json::to_string_pretty(&doc).expect("serialize ops doc")
}

/// One OPS task object per line — a line-delimited variant of the JSON
/// export that is easy to stream and diff.
pub fn serialize_jsonl(tasks: &[Task]) -> String {
    let mut out = String::new();
    for t in tasks {
        out.push_str(&serde_json::to_string(&task_to_ops(t)).expect("ops"));
        out.push('\n');
    }
    out
}

/// Parse a JSONL export (one OPS task object per line).
pub fn parse_jsonl_doc(text: &str) -> Result<Vec<OpsTask>, String> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let ops: OpsTask =
            serde_json::from_str(line).map_err(|e| format!("jsonl line {}: {e}", i + 1))?;
        out.push(ops);
    }
    if out.is_empty() {
        return Err("jsonl file has no tasks".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::super::{fixture_task, ops_to_task, EXPORT_FIXTURE};
    use super::*;
    use crate::model::{Freq, Status};

    #[test]
    fn validates_well_formed_doc() {
        let doc = validate_doc(EXPORT_FIXTURE).expect("valid");
        assert_eq!(doc.tasks.len(), 1);
        assert_eq!(doc.tasks[0].metadata["due_local"], "2026-03-15");
    }

    #[test]
    fn rejects_missing_status_and_subtasks() {
        // Missing required field surfaces as a shape error naming it.
        let bad = r#"{ "tasks": [{ "title": "x", "subtasks": [{}] }] }"#;
        let errs = validate_doc(bad).expect_err("must fail");
        assert!(errs.iter().any(|e| e.contains("status")));
        // Subtasks present with all required fields → per-task rejection.
        let bad2 = r#"{ "tasks": [{ "title": "x", "status": "open", "subtasks": [{}] }] }"#;
        let errs = validate_doc(bad2).expect_err("must fail");
        assert!(errs.iter().any(|e| e.contains("subtasks")));
    }

    #[test]
    fn rejects_bad_metadata_keys() {
        let bad =
            r#"{ "tasks": [{ "title": "x", "status": "open", "metadata": { "Bad-Key": 1 } }] }"#;
        let errs = validate_doc(bad).expect_err("must fail");
        assert!(errs.iter().any(|e| e.contains("metadata key")));
    }

    #[test]
    fn rejects_duplicate_ids_and_missing_tasks_array() {
        let bad = r#"{ "tasks": [{ "id": "a", "title": "x", "status": "open" }, { "id": "a", "title": "y", "status": "open" }] }"#;
        let errs = validate_doc(bad).expect_err("must fail");
        assert!(errs.iter().any(|e| e.contains("duplicate id")));
        let errs = validate_doc(r#"{"nope": []}"#).expect_err("must fail");
        assert!(errs.iter().any(|e| e.contains("tasks")));
    }

    #[test]
    fn roundtrip_preserves_unknown_metadata() {
        let raw = r#"{ "tasks": [{ "title": "x", "status": "open", "id": "a",
          "metadata": { "favorite_thing": "cats" } }] }"#;
        let doc = validate_doc(raw).unwrap();
        let t = ops_to_task(
            doc.tasks.into_iter().next().unwrap(),
            "2026-03-13T10:00:00.000Z",
        );
        assert_eq!(t.metadata["favorite_thing"], "cats");
        let back = serialize_export(&[t.clone()]);
        let doc2 = validate_doc(&back).unwrap();
        let t2 = ops_to_task(
            doc2.tasks.into_iter().next().unwrap(),
            "2026-03-13T10:00:00.000Z",
        );
        assert_eq!(t2.metadata["favorite_thing"], "cats");
        assert_eq!(t2.id, t.id);
        assert_eq!(t2.title, t.title);
    }

    #[test]
    fn import_generates_id_when_missing() {
        let raw = r#"{ "tasks": [{ "title": "x", "status": "open" }] }"#;
        let doc = validate_doc(raw).unwrap();
        let t = ops_to_task(doc.tasks.into_iter().next().unwrap(), "now");
        assert!(!t.id.is_empty());
    }

    #[test]
    fn full_roundtrip_keeps_fields() {
        let doc = validate_doc(EXPORT_FIXTURE).unwrap();
        let t = ops_to_task(
            doc.tasks.into_iter().next().unwrap(),
            "2026-03-13T10:00:00.000Z",
        );
        assert_eq!(t.title, "Buy milk");
        assert_eq!(t.priority.as_deref(), Some("high"));
        assert_eq!(t.tags, vec!["errands"]);
        assert_eq!(t.location.as_deref(), Some("shop"));
        assert_eq!(t.due_local.as_deref(), Some("2026-03-15"));
        assert_eq!(t.recurrence.as_ref().unwrap().freq, Freq::Weekly);
        assert_eq!(t.status, Status::Open);
        // Exporting again keeps the same due_local (no UTC re-derivation drift).
        let back = serialize_export(&[t]);
        let doc2 = validate_doc(&back).unwrap();
        assert_eq!(doc2.tasks[0].metadata["due_local"], "2026-03-15");
    }

    #[test]
    fn json_exports_declare_ops_version() {
        // Spec rule 6: roots carry the version their subtree is modeled
        // after — every task in the flat export is a root.
        let out = serialize_export(&[fixture_task()]);
        assert!(out.contains("\"version\": \"0\""));
        let jsonl = serialize_jsonl(&[fixture_task()]);
        assert!(jsonl.starts_with("{\"version\":\"0\""));
    }

    #[test]
    fn jsonl_is_one_object_per_line() {
        let out = serialize_jsonl(&[fixture_task(), fixture_task()]);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 2);
        let v: Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(v["title"], "Buy milk");
    }

    #[test]
    fn jsonl_roundtrips() {
        let t = fixture_task();
        let out = serialize_jsonl(&[t.clone()]);
        let parsed = parse_jsonl_doc(&out).unwrap();
        let back = ops_to_task(
            parsed.into_iter().next().unwrap(),
            "2026-03-13T10:00:00.000Z",
        );
        assert_eq!(back.title, t.title);
        assert_eq!(back.due_local, t.due_local);
        assert_eq!(back.tags, t.tags);
    }
}
