//! OPS tasks into todo lines, appended to a document.

use ops_lib::v0::{Format, MetadataValue, Status, Task as OpsTask};

use crate::doc::Doc;
use crate::error::Error;

/// Append every task in an OPS document, depth first, and say how many lines
/// were added. A task's `title` becomes the body verbatim; only a valid
/// `metadata.priority` letter is prepended back as the positional token.
pub fn append(doc: &mut Doc, bytes: &str, format: Format) -> Result<usize, Error> {
    let tasks = ops_lib::v0::read(format, bytes).map_err(|error| Error::Ops(error.to_string()))?;
    let mut at = doc.lines().len();
    let mut added = 0;
    for task in &tasks {
        added += insert(doc, task, &mut at, 0);
    }
    Ok(added)
}

fn insert(doc: &mut Doc, task: &OpsTask, at: &mut usize, indent: usize) -> usize {
    let Some(inverse) = doc.insert_at(*at, indent, &body(task)) else {
        return 0;
    };
    let crate::model::Inverse::Insert { line, .. } = inverse else {
        return 0;
    };
    *at += 1;
    if task.status == Status::Done {
        doc.check(line);
    }
    let mut added = 1;
    for child in &task.subtasks {
        added += insert(doc, child, at, indent + 2);
    }
    added
}

/// The title, with the priority put back in front when metadata carries one.
fn body(task: &OpsTask) -> String {
    match priority(task.metadata.as_ref()) {
        Some(prio) => format!("({}) {}", prio.letter(), task.title),
        None => task.title.clone(),
    }
}

/// `metadata.priority`, when it is a single A–D letter. Anything else is not a
/// priority this tool knows, so it is ignored rather than written back.
fn priority(metadata: Option<&ops_lib::v0::Metadata>) -> Option<crate::model::Prio> {
    let MetadataValue::String(letter) = metadata?.get("priority")? else {
        return None;
    };
    let mut characters = letter.chars();
    let (Some(letter), None) = (characters.next(), characters.next()) else {
        return None;
    };
    crate::model::Prio::from_letter(letter)
}

#[cfg(test)]
mod tests {
    use ops_lib::v0::Format;

    use super::append;
    use crate::doc::Doc;

    fn ops(task: &serde_json::Value) -> String {
        serde_json::to_string(task).unwrap()
    }

    #[test]
    fn import_writes_the_title_verbatim_and_prepends_the_priority() {
        let mut doc = Doc::parse("");
        let bytes = ops(&serde_json::json!({
            "title": "a @x #y",
            "status": "open",
            "metadata": { "priority": "B" }
        }));
        assert_eq!(append(&mut doc, &bytes, Format::Json).unwrap(), 1);
        assert_eq!(doc.render(), "- [ ] (B) a @x #y\n");
    }

    #[test]
    fn import_ignores_a_priority_outside_a_to_d() {
        let mut doc = Doc::parse("");
        let bytes = ops(&serde_json::json!({
            "title": "a",
            "status": "open",
            "metadata": { "priority": "high" }
        }));
        append(&mut doc, &bytes, Format::Json).unwrap();
        assert_eq!(doc.render(), "- [ ] a\n");
    }

    #[test]
    fn import_marks_done_tasks() {
        let mut doc = Doc::parse("");
        let bytes = ops(&serde_json::json!({ "title": "done already", "status": "done" }));
        append(&mut doc, &bytes, Format::Json).unwrap();
        assert_eq!(doc.render(), "- [x] done already\n");
    }

    #[test]
    fn import_appends_and_recurses_into_subtasks() {
        let mut doc = Doc::parse("- [ ] first\n");
        let bytes = serde_json::to_string(&serde_json::json!([
            {
                "title": "parent",
                "status": "open",
                "subtasks": [
                    { "title": "child", "status": "done" },
                    { "title": "child two", "status": "open" }
                ]
            }
        ]))
        .unwrap();
        assert_eq!(append(&mut doc, &bytes, Format::Json).unwrap(), 3);
        assert_eq!(
            doc.render(),
            "- [ ] first\n- [ ] parent\n  - [x] child\n  - [ ] child two\n"
        );
    }

    #[test]
    fn import_of_nothing_adds_nothing() {
        let mut doc = Doc::parse("- [ ] a\n");
        assert_eq!(append(&mut doc, "[]", Format::Json).unwrap(), 0);
        assert_eq!(doc.render(), "- [ ] a\n");
    }

    #[test]
    fn a_document_that_is_not_ops_is_an_error() {
        let mut doc = Doc::parse("");
        assert!(matches!(
            append(&mut doc, "{not json", Format::Json),
            Err(crate::error::Error::Ops(_))
        ));
    }

    #[test]
    fn round_trip_export_then_import_is_stable() {
        use std::fs;

        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("TODOs.md");
        let source = "- [ ] (A) @dave fix you #bug\n- [x] plain\n";
        fs::write(&path, source).unwrap();
        let files = crate::discover::scan(&path).unwrap().entries;
        let exported = crate::ops::write(&files, Format::Json).unwrap();
        let mut imported = Doc::parse("");
        append(&mut imported, &exported.text, Format::Json).unwrap();
        assert_eq!(imported.render(), source);
    }
}
