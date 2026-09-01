//! OPS import/export (MVP.md §8, ADR-0007) — one module per serialization:
//! [`json`] (tree JSON + JSONL), [`csv`] (the spec's flat dialect),
//! [`markdown`] (checklist with metadata comments). The shared pieces live
//! here: the OPS document types, general parsing helpers, and the
//! Task ↔ OpsTask mapping every format's import and export routes through.
//! `metadata` is preserved verbatim so round-trips are lossless (OPS
//! conformance); `due_local` rides along so local calendar dates survive
//! (ADR-0004).

mod csv;
mod json;
mod markdown;

pub use csv::{parse_csv_doc, serialize_csv};
pub use json::{parse_jsonl_doc, serialize_export, serialize_jsonl, validate_doc};
pub use markdown::{parse_markdown_doc, serialize_markdown};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::model::{new_id, Status, Task};

/// The OPS Specifications version this app's exports declare (spec rule 6:
/// roots carry the version their subtree is modeled after; every task in
/// this app's flat exports is a root). Mirrors the draft spec's frontmatter
/// (`version: "0"`, status: draft) — bump when the spec releases.
pub const OPS_VERSION: &str = "0";

#[derive(Debug, Serialize, Deserialize)]
pub struct OpsDoc {
    pub tasks: Vec<OpsTask>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpsTask {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub title: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default)]
    pub subtasks: Vec<Value>,
    #[serde(default)]
    pub metadata: Map<String, Value>,
}

#[derive(Debug, Default)]
pub struct ImportResult {
    pub imported: usize,
    pub updated: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

/// OPS rule 5: keys must match `^[a-z0-9_]{3,}$`. Shared by the JSON
/// validator and the CSV extra-column path.
pub(crate) fn valid_metadata_key(k: &str) -> bool {
    k.len() >= 3
        && k.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn set_meta(m: &mut Map<String, Value>, k: &str, v: Option<&String>) {
    match v {
        Some(v) => {
            m.insert(k.into(), Value::String(v.clone()));
        }
        None => {
            m.remove(k);
        }
    }
}

/// OPS JSON doc — flat MVP tasks, `metadata` preserved verbatim so
/// round-trips are lossless (OPS conformance). Child-id lists (flat-format
/// `subtasks`) ride in metadata through the MVP-flat app.
pub fn task_to_ops(t: &Task) -> OpsTask {
    let mut metadata: Map<String, Value> = t.metadata.as_object().cloned().unwrap_or_default();
    // Child-id lists are an OPS column in flat formats, not metadata — but
    // the MVP app is flat, so they live in metadata; export them as the
    // column so flat round-trips stay lossless.
    let subtasks = metadata
        .remove("subtasks")
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();
    set_meta(&mut metadata, "due_date", t.due_date.as_ref());
    // due_local rides along so local-calendar dates survive round trips
    // (re-deriving from the UTC instant would shift a day for non-UTC users).
    set_meta(&mut metadata, "due_local", t.due_local.as_ref());
    set_meta(&mut metadata, "priority", t.priority.as_ref());
    set_meta(&mut metadata, "location", t.location.as_ref());
    metadata.insert(
        "tags".into(),
        Value::Array(t.tags.iter().map(|s| Value::String(s.clone())).collect()),
    );
    if let Some(r) = &t.recurrence {
        metadata.insert(
            "recurrence".into(),
            serde_json::to_value(r).expect("recurrence"),
        );
    }
    set_meta(&mut metadata, "created_at", Some(&t.created_at));
    set_meta(&mut metadata, "updated_at", Some(&t.updated_at));
    set_meta(&mut metadata, "completed_at", t.completed_at.as_ref());
    OpsTask {
        version: Some(OPS_VERSION.to_string()),
        id: Some(t.id.clone()),
        title: t.title.clone(),
        status: t.status.as_str().to_string(),
        notes: t.notes.clone(),
        subtasks,
        metadata,
    }
}

/// Map a validated OPS task to a DB Task; unknown metadata rides along verbatim.
pub fn ops_to_task(ops: OpsTask, now: &str) -> Task {
    let mut meta = ops.metadata;
    // Flat-format child-id lists ride along verbatim (OPS Transformer: keep
    // the graph even when the MVP doesn't render it).
    if !ops.subtasks.is_empty() {
        meta.insert("subtasks".into(), Value::Array(ops.subtasks));
    }
    let s = |k: &str| meta.get(k).and_then(|v| v.as_str()).map(str::to_string);
    let tags = meta
        .get("tags")
        .and_then(|v| match v {
            Value::Array(a) => Some(
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect(),
            ),
            // Foreign/legacy CSVs carry tags as a comma-separated string.
            Value::String(s) => Some(
                s.split(',')
                    .filter(|t| !t.is_empty())
                    .map(str::to_string)
                    .collect(),
            ),
            _ => None,
        })
        .unwrap_or_default();
    Task {
        id: ops.id.filter(|s| !s.is_empty()).unwrap_or_else(new_id),
        title: ops.title,
        status: Status::parse(&ops.status),
        notes: ops.notes,
        parent_id: None,
        created_at: s("created_at").unwrap_or_else(|| now.to_string()),
        updated_at: s("updated_at").unwrap_or_else(|| now.to_string()),
        completed_at: s("completed_at"),
        due_date: s("due_date"),
        due_local: s("due_local")
            .or_else(|| s("due_date").and_then(|d| d.get(..10).map(str::to_string))),
        priority: s("priority"),
        location: s("location"),
        tags,
        recurrence: meta
            .get("recurrence")
            .and_then(|v| serde_json::from_value(v.clone()).ok()),
        metadata: Value::Object(meta),
    }
}

// ── shared test fixture (used by the domain test modules) ────────────────

#[cfg(test)]
const EXPORT_FIXTURE: &str = r#"{
  "tasks": [{
    "id": "t1", "title": "Buy milk", "status": "open",
    "notes": null, "subtasks": [],
    "metadata": { "due_date": "2026-03-14T23:00:00.000Z", "due_local": "2026-03-15",
      "priority": "high", "tags": ["errands"], "location": "shop",
      "recurrence": { "freq": "weekly", "interval": 1 },
      "created_at": "2026-03-13T10:00:00.000Z", "updated_at": "2026-03-13T10:00:00.000Z",
      "completed_at": null }
  }]
}"#;

/// A task decoded from the shared export fixture, with every typed field
/// populated — the round-trip baseline for all format tests.
#[cfg(test)]
pub(crate) fn fixture_task() -> Task {
    let doc = json::validate_doc(EXPORT_FIXTURE).unwrap();
    ops_to_task(
        doc.tasks.into_iter().next().unwrap(),
        "2026-03-13T10:00:00.000Z",
    )
}
