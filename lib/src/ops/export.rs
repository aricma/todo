//! Todo lines out to OPS tasks.

use ops_lib::v0::{Format, Metadata, MetadataValue, Status, Task as OpsTask};

use super::Exported;
use crate::discover::Entry;
use crate::doc::{spans, Doc};
use crate::error::Error;
use crate::model::Task;
use crate::store;

/// Every task in every given file, as one OPS document.
pub fn write(files: &[Entry], format: Format) -> Result<Exported, Error> {
    let mut exported = Exported::default();
    let mut tasks = Vec::new();
    for file in files.iter().filter(|file| !file.too_long) {
        let loaded = store::load(&file.path)?;
        let doc = Doc::parse(&loaded.text);
        for task in doc.tasks() {
            let parts = spans::parts(&task.body);
            let where_ = format!("{}:{}", file.path.display(), task.line + 1);
            if parts.title.trim().is_empty() {
                exported.skipped.push(format!("{where_}: no title"));
                continue;
            }
            let title = title(&task);
            // OPS bounds a title at 1 KiB; a todo line longer than that is not
            // something to truncate silently.
            if title.len() > ops_lib::v0::limits::MAX_TITLE_BYTES {
                exported.skipped.push(format!(
                    "{where_}: title is {} bytes, the OPS limit is {}",
                    title.len(),
                    ops_lib::v0::limits::MAX_TITLE_BYTES
                ));
                continue;
            }
            let mut ops = OpsTask::new(title, status(&task));
            ops.id = Some(format!("md-{}", tasks.len() + 1));
            let metadata = metadata(&parts);
            if !metadata.is_empty() {
                ops.metadata = Some(metadata);
            }
            tasks.push(ops);
        }
    }
    exported.text =
        ops_lib::v0::write(format, &tasks).map_err(|error| Error::Ops(error.to_string()))?;
    Ok(exported)
}

/// The body without the positional priority token: that is metadata.
fn title(task: &Task) -> String {
    if task.prio.is_some() {
        task.body[3..].trim_start().to_owned()
    } else {
        task.body.clone()
    }
}

fn status(task: &Task) -> Status {
    if task.done {
        Status::Done
    } else {
        Status::Open
    }
}

/// The spec's recommended keys: `priority`, `tags`, and `assignee` for one
/// assignee or `assignees` for several, mirroring `attachment`/`attachments`.
fn metadata(parts: &crate::model::Parts) -> Metadata {
    let mut metadata = Metadata::new();
    if let Some(prio) = parts.prio {
        metadata.insert(
            "priority".to_owned(),
            MetadataValue::String(prio.letter().to_string()),
        );
    }
    if !parts.tags.is_empty() {
        metadata.insert(
            "tags".to_owned(),
            list(parts.tags.iter().map(|tag| tag.as_str().to_owned())),
        );
    }
    let assignees: Vec<String> = parts
        .assignees
        .iter()
        .map(|assignee| assignee.as_str().to_owned())
        .collect();
    match assignees.len() {
        0 => {}
        1 => {
            metadata.insert(
                "assignee".to_owned(),
                MetadataValue::String(assignees[0].clone()),
            );
        }
        _ => {
            metadata.insert("assignees".to_owned(), list(assignees.into_iter()));
        }
    }
    metadata
}

fn list(values: impl Iterator<Item = String>) -> MetadataValue {
    MetadataValue::List(values.map(MetadataValue::String).collect())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use ops_lib::v0::Format;
    use serde_json::Value;

    use super::write;
    use crate::discover::scan;

    fn entries(root: &Path, sources: &[(&str, &str)]) -> Vec<crate::discover::Entry> {
        for (name, text) in sources {
            fs::write(root.join(name), text).unwrap();
        }
        scan(root).unwrap().entries
    }

    /// ops-lib writes a bare object for one task and an array for several, so
    /// read it the way ops-lib itself does.
    fn json(exported: &super::Exported) -> Vec<Value> {
        match serde_json::from_str(&exported.text).unwrap() {
            Value::Array(items) => items,
            single => vec![single],
        }
    }

    #[test]
    fn the_ops_title_drops_the_priority_token() {
        let tmp = tempfile::tempdir().unwrap();
        let files = entries(
            tmp.path(),
            &[("TODOs.md", "- [x] (A) @dave fix you #bug\n")],
        );
        let exported = write(&files, Format::Json).unwrap();
        let tasks = json(&exported);
        let task = &tasks[0];
        assert_eq!(task["title"], "@dave fix you #bug");
        assert_eq!(task["status"], "done");
        assert_eq!(task["id"], "md-1");
        assert_eq!(task["metadata"]["priority"], "A");
        assert_eq!(task["metadata"]["tags"], serde_json::json!(["bug"]));
        assert_eq!(task["metadata"]["assignee"], "dave");
    }

    #[test]
    fn a_body_without_a_priority_has_no_priority_metadata() {
        let tmp = tempfile::tempdir().unwrap();
        let files = entries(tmp.path(), &[("TODOs.md", "- [ ] plain\n")]);
        let exported = write(&files, Format::Json).unwrap();
        let tasks = json(&exported);
        let task = &tasks[0];
        assert_eq!(task["status"], "open");
        assert!(task.get("metadata").is_none(), "{task}");
    }

    #[test]
    fn several_assignees_use_the_plural_key() {
        let tmp = tempfile::tempdir().unwrap();
        let files = entries(tmp.path(), &[("TODOs.md", "- [ ] ship @sam @aricma #x\n")]);
        let exported = write(&files, Format::Json).unwrap();
        let tasks = json(&exported);
        let metadata = &tasks[0]["metadata"];
        assert_eq!(metadata["assignees"], serde_json::json!(["sam", "aricma"]));
        assert_eq!(metadata["tags"], serde_json::json!(["x"]));
        assert!(metadata.get("assignee").is_none(), "{metadata}");
    }

    #[test]
    fn export_skips_a_task_with_no_title_and_reports_it() {
        let tmp = tempfile::tempdir().unwrap();
        let files = entries(
            tmp.path(),
            &[("TODOs.md", "# t\n- [ ] (A) #x @y\n- [ ] kept\n")],
        );
        let exported = write(&files, Format::Json).unwrap();
        assert_eq!(exported.skipped.len(), 1);
        assert!(
            exported.skipped[0].contains("TODOs.md:2: no title"),
            "{:?}",
            exported.skipped
        );
        assert_eq!(json(&exported).len(), 1);
    }

    #[test]
    fn a_title_over_the_ops_limit_is_skipped_and_reported() {
        let tmp = tempfile::tempdir().unwrap();
        let long = "a".repeat(2_000);
        let files = entries(
            tmp.path(),
            &[("TODOs.md", &format!("- [ ] {long}\n- [ ] kept\n"))],
        );
        let exported = write(&files, Format::Json).unwrap();
        assert_eq!(exported.skipped.len(), 1);
        assert!(
            exported.skipped[0].contains("the OPS limit is 1024"),
            "{:?}",
            exported.skipped
        );
        assert_eq!(json(&exported).len(), 1);
    }

    #[test]
    fn every_task_gets_an_id_so_flat_formats_can_write() {
        let tmp = tempfile::tempdir().unwrap();
        let files = entries(tmp.path(), &[("TODOs.md", "- [ ] a\n- [ ] b\n")]);
        for format in [Format::Csv, Format::Jsonl, Format::Yaml] {
            let exported = write(&files, format).expect("flat formats need ids");
            assert!(!exported.text.is_empty());
        }
    }

    #[test]
    fn every_files_tasks_land_in_one_document() {
        let tmp = tempfile::tempdir().unwrap();
        let files = entries(
            tmp.path(),
            &[("TODOs.md", "- [ ] a\n"), ("TASKS.md", "- [ ] b\n")],
        );
        let exported = write(&files, Format::Json).unwrap();
        assert_eq!(json(&exported).len(), 2);
    }
}
