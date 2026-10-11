//! OPS import/export (MVP.md §8, ADR-0007): the app's `Task` converted to and
//! from the OPS task and read/written by the reference `ops-lib` crate
//! (submodule: `open-productivity-standard/ops`). The serializations live in
//! that library; this module is the app-side seam — the mapping, format
//! detection, and the metadata-key [`repair`] that makes foreign files
//! importable.
//!
//! Formats: JSON, YAML, JSONL, CSV. Markdown is a draft in the standard and
//! not claimed by `ops-lib`, so the app neither imports nor exports it.
//!
//! `metadata` rides verbatim so a round trip keeps what the app does not
//! understand (OPS conformance). App fields the standard recommends a key for
//! become metadata on export and typed fields on import; see `to_metadata` /
//! `flatten`.

mod repair;

#[cfg(test)]
mod tests;

pub use repair::Repair;

use repair::valid_key;

use std::collections::{HashMap, HashSet};

use ops_lib::v0::{self, Format, Metadata, Task as OpsTask};
use serde_json::Value;

use crate::model::{new_id, Recurrence, Status, Task};

/// The OPS Specifications version this app writes (spec: roots carry the
/// version their subtree is modeled after).
const OPS_VERSION: &str = "0";

/// An export format: the name the UI and `ops-lib` share, plus its file type.
pub struct ExportFormat {
    pub name: &'static str,
    pub extension: &'static str,
    pub mime: &'static str,
}

/// The formats the export offers; JSON first (the default).
pub const EXPORT_FORMATS: &[ExportFormat] = &[
    ExportFormat {
        name: "json",
        extension: "json",
        mime: "application/json",
    },
    ExportFormat {
        name: "yaml",
        extension: "yaml",
        mime: "application/yaml",
    },
    ExportFormat {
        name: "jsonl",
        extension: "jsonl",
        mime: "application/jsonl",
    },
    ExportFormat {
        name: "csv",
        extension: "csv",
        mime: "text/csv",
    },
];

/// What one file became in the store, plus the key renames the import applied
/// so the UI can say what it fixed.
#[derive(Debug)]
pub struct ImportOutcome {
    pub tasks: Vec<Task>,
    pub repairs: Vec<String>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ImportResult {
    pub imported: usize,
    pub updated: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
    pub repairs: Vec<String>,
}

impl ImportResult {
    /// The one-line report the app shows: what the file became in the store.
    pub fn summary(&self) -> String {
        format!(
            "imported {}, updated {}, skipped {}",
            self.imported, self.updated, self.skipped
        )
    }

    /// Whether the file brought nothing at all — an empty or all-duplicate
    /// document, which is worth saying out loud but is not a failure.
    pub fn is_empty(&self) -> bool {
        self.imported == 0 && self.updated == 0
    }
}

/// Read a document in any supported format: detect, repair invalid metadata
/// keys, read with `ops-lib`, convert to the app's tasks.
pub fn read_tasks(text: &str, now: &str) -> Result<ImportOutcome, String> {
    let format = detect_format(text);
    let (repaired, repairs) = repair::repair(format, text)?;
    let tasks = v0::read(format, &repaired).map_err(|e| e.to_string())?;
    Ok(ImportOutcome {
        tasks: from_ops(tasks, now),
        repairs: repairs.iter().map(Repair::describe).collect(),
    })
}

/// Write the app's tasks as an OPS document in `format`.
pub fn write_tasks(format: &str, tasks: &[Task]) -> Result<String, String> {
    let format =
        Format::from_name(format).ok_or_else(|| format!("unsupported format `{format}`"))?;
    v0::write(format, &to_ops(tasks)).map_err(|e| e.to_string())
}

// ── Format detection ────────────────────────────────────────────────────────

fn detect_format(text: &str) -> Format {
    if looks_like_csv(text) {
        return Format::Csv;
    }
    let trimmed = text.trim();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        if serde_json::from_str::<Value>(trimmed).is_ok() {
            return Format::Json;
        }
        if text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .all(|line| serde_json::from_str::<Value>(line).is_ok())
        {
            return Format::Jsonl;
        }
    }
    Format::Yaml
}

/// A first line with a `status` cell is a CSV header (the OPS dialect or a
/// foreign one).
fn looks_like_csv(text: &str) -> bool {
    text.lines()
        .next()
        .map(|header| {
            header
                .split(',')
                .any(|cell| cell.trim().eq_ignore_ascii_case("status"))
        })
        .unwrap_or(false)
}

// ── Task → OPS ──────────────────────────────────────────────────────────────

fn to_ops(tasks: &[Task]) -> Vec<OpsTask> {
    let ids: HashSet<&str> = tasks.iter().map(|task| task.id.as_str()).collect();
    let mut children: HashMap<&str, Vec<&Task>> = HashMap::new();
    let mut roots: Vec<&Task> = Vec::new();
    for task in tasks {
        match task.parent_id.as_deref() {
            Some(parent) if parent != task.id.as_str() && ids.contains(parent) => {
                children.entry(parent).or_default().push(task);
            }
            _ => roots.push(task),
        }
    }
    roots
        .iter()
        .map(|task| build_ops(task, &children, true))
        .collect()
}

fn build_ops(task: &Task, children: &HashMap<&str, Vec<&Task>>, root: bool) -> OpsTask {
    let subtasks = children
        .get(task.id.as_str())
        .map(|kids| {
            kids.iter()
                .map(|kid| build_ops(kid, children, false))
                .collect()
        })
        .unwrap_or_default();
    OpsTask {
        title: task.title.clone(),
        status: match task.status {
            Status::Open => v0::Status::Open,
            Status::Done => v0::Status::Done,
        },
        id: Some(task.id.clone()).filter(|id| !id.is_empty()),
        version: root.then(|| OPS_VERSION.to_string()),
        notes: task
            .notes
            .as_deref()
            .filter(|notes| !notes.is_empty())
            .map(str::to_string),
        metadata: to_metadata(task),
        subtasks,
    }
}

/// The app fields as OPS metadata: the spec's recommended keys, plus the app's
/// own `metadata` (kept verbatim, except keys that break the OPS charset and
/// so cannot be written at all).
fn to_metadata(task: &Task) -> Option<Metadata> {
    let mut map = task.metadata.as_object().cloned().unwrap_or_default();
    map.retain(|key, _| valid_key(key));

    let mut set = |key: &str, value: Option<&String>| match value {
        Some(value) => {
            map.insert(key.to_string(), Value::String(value.clone()));
        }
        None => {
            map.remove(key);
        }
    };
    set("due_date", task.due_date.as_ref());
    set("due_local", task.due_local.as_ref());
    set("priority", task.priority.as_ref());
    set("location", task.location.as_ref());
    set("completed_at", task.completed_at.as_ref());
    map.insert("created_at".into(), Value::String(task.created_at.clone()));
    map.insert("updated_at".into(), Value::String(task.updated_at.clone()));
    if !task.tags.is_empty() {
        map.insert(
            "tags".into(),
            Value::Array(task.tags.iter().cloned().map(Value::String).collect()),
        );
    }
    if let Some(value) = task
        .recurrence
        .as_ref()
        .and_then(|recurrence| serde_json::to_value(recurrence).ok())
    {
        map.insert("recurrence".into(), value);
    }

    let metadata: Metadata = serde_json::from_value(Value::Object(map)).ok()?;
    (!metadata.is_empty()).then_some(metadata)
}

// ── OPS → Task ──────────────────────────────────────────────────────────────

fn from_ops(tasks: Vec<OpsTask>, now: &str) -> Vec<Task> {
    let mut out = Vec::new();
    for task in tasks {
        flatten(task, None, now, &mut out);
    }
    out
}

/// One OPS task and its subtree, pre-order: the tree becomes flat tasks with
/// `parent_id`, which is what the app stores.
fn flatten(task: OpsTask, parent_id: Option<&str>, now: &str, out: &mut Vec<Task>) {
    let OpsTask {
        title,
        status,
        id,
        version: _,
        notes,
        metadata,
        subtasks,
    } = task;
    let id = id.filter(|id| !id.is_empty()).unwrap_or_else(new_id);
    let mut meta = metadata
        .and_then(|metadata| serde_json::to_value(&metadata).ok())
        .and_then(|value| match value {
            Value::Object(map) => Some(map),
            _ => None,
        })
        .unwrap_or_default();

    let due_date = take_string(&mut meta, "due_date");
    let due_local = take_string(&mut meta, "due_local").or_else(|| {
        due_date
            .as_deref()
            .and_then(|date| date.get(..10))
            .map(str::to_string)
    });
    let priority = take_string(&mut meta, "priority");
    let location = take_string(&mut meta, "location");
    let completed_at = take_string(&mut meta, "completed_at");
    let created_at = take_string(&mut meta, "created_at").unwrap_or_else(|| now.to_string());
    let updated_at = take_string(&mut meta, "updated_at").unwrap_or_else(|| now.to_string());
    let tags = meta
        .remove("tags")
        .map(|value| match value {
            Value::Array(items) => items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect(),
            // Foreign CSVs carry tags as a comma-separated string.
            Value::String(text) => text
                .split(',')
                .map(str::trim)
                .filter(|tag| !tag.is_empty())
                .map(str::to_string)
                .collect(),
            _ => Vec::new(),
        })
        .unwrap_or_default();
    let recurrence = meta
        .remove("recurrence")
        .and_then(|value| serde_json::from_value::<Recurrence>(value).ok());

    out.push(Task {
        id: id.clone(),
        title,
        status: Status::parse(status.as_str()),
        notes,
        parent_id: parent_id.map(str::to_string),
        created_at,
        updated_at,
        completed_at,
        due_date,
        due_local,
        priority,
        location,
        tags,
        recurrence,
        metadata: Value::Object(meta),
    });

    for child in subtasks {
        flatten(child, Some(&id), now, out);
    }
}

fn take_string(map: &mut serde_json::Map<String, Value>, key: &str) -> Option<String> {
    map.remove(key)
        .and_then(|value| value.as_str().map(str::to_string))
}
