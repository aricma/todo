//! Integration tests for the OPS facade. `ops-lib` is a third-party
//! dependency with its own conformance suite, so these test the seam we own:
//! conversion, format detection, and the metadata-key repair.

use ops_lib::v0::Format;
use serde_json::{json, Value};

use super::*;
use crate::model::{Freq, Recurrence};

const NOW: &str = "2026-03-13T10:00:00.000Z";

fn full_task() -> Task {
    Task {
        id: "t-1".into(),
        title: "Buy milk ✓".into(),
        status: Status::Done,
        notes: Some("two litres".into()),
        parent_id: None,
        created_at: "2026-03-13T10:00:00.000Z".into(),
        updated_at: "2026-03-13T11:00:00.000Z".into(),
        completed_at: Some("2026-03-13T11:00:00.000Z".into()),
        due_date: Some("2026-03-14T23:00:00.000Z".into()),
        due_local: Some("2026-03-15".into()),
        priority: Some("high".into()),
        location: Some("shop".into()),
        tags: vec!["errands".into(), "food".into()],
        recurrence: Some(Recurrence {
            freq: Freq::Weekly,
            interval: 2,
            weekdays: None,
        }),
        metadata: json!({ "due_time": "09:00" }),
    }
}

fn value(task: &Task) -> Value {
    serde_json::to_value(task).expect("task serializes")
}

#[test]
fn every_format_round_trips_a_full_task() {
    let task = full_task();
    for format in ["json", "yaml", "jsonl", "csv"] {
        let text = write_tasks(format, std::slice::from_ref(&task)).expect(format);
        let outcome = read_tasks(&text, NOW).expect(format);
        assert_eq!(outcome.tasks.len(), 1, "{format}: {text}");
        assert_eq!(value(&outcome.tasks[0]), value(&task), "{format}: {text}");
        assert!(outcome.repairs.is_empty(), "{format}");
    }
}

#[test]
fn empty_notes_are_absent_again() {
    let mut task = full_task();
    task.notes = Some(String::new());
    let text = write_tasks("json", &[task]).unwrap();
    let outcome = read_tasks(&text, NOW).unwrap();
    assert_eq!(outcome.tasks[0].notes, None);
}

#[test]
fn a_task_without_an_id_gets_one() {
    let tasks = read_tasks(r#"[{"title":"t","status":"open"}]"#, NOW)
        .unwrap()
        .tasks;
    assert!(!tasks[0].id.is_empty());
}

#[test]
fn a_comma_tag_string_imports_as_a_list() {
    let tasks = read_tasks(
        r#"[{"title":"t","status":"open","metadata":{"tags":"a, b"}}]"#,
        NOW,
    )
    .unwrap()
    .tasks;
    assert_eq!(tasks[0].tags, vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn jsonl_and_yaml_are_detected() {
    let jsonl = "{\"title\":\"a\",\"status\":\"open\"}\n{\"title\":\"b\",\"status\":\"done\"}\n";
    assert_eq!(read_tasks(jsonl, NOW).unwrap().tasks.len(), 2);
    let yaml = "title: y\nstatus: open\n";
    assert_eq!(read_tasks(yaml, NOW).unwrap().tasks[0].title, "y");
}

#[test]
fn hierarchy_survives_flatten_and_rebuild() {
    let doc = r#"[{"title":"parent","status":"open","id":"p",
        "subtasks":[{"title":"child","status":"open","id":"c"}]}]"#;
    let tasks = read_tasks(doc, NOW).unwrap().tasks;
    assert_eq!(tasks[0].parent_id, None);
    assert_eq!(tasks[1].parent_id.as_deref(), Some("p"));

    let text = write_tasks("json", &tasks).unwrap();
    let again = read_tasks(&text, NOW).unwrap().tasks;
    assert_eq!(again.len(), 2);
    assert_eq!(again[1].parent_id.as_deref(), Some("p"));
}

#[test]
fn invalid_keys_are_repaired_and_reported() {
    let doc = r#"[{"title":"t","status":"open","metadata":
        {"Task Name":"x","my.key":"w","P1":"z","weird!!key":"v"}}]"#;
    let outcome = read_tasks(doc, NOW).unwrap();
    let meta = &outcome.tasks[0].metadata;
    assert_eq!(meta["task_name"], json!("x"));
    assert_eq!(meta["my_key"], json!("w"));
    assert_eq!(meta["_p1"], json!("z"));
    assert_eq!(meta["weird_key"], json!("v"));
    assert_eq!(outcome.repairs.len(), 4);
}

#[test]
fn a_valid_key_is_never_renamed_and_a_collision_is_suffixed() {
    let doc = r#"[{"title":"t","status":"open","metadata":{"Task Name":"a","task_name":"b"}}]"#;
    let outcome = read_tasks(doc, NOW).unwrap();
    let meta = &outcome.tasks[0].metadata;
    assert_eq!(meta["task_name"], json!("b"));
    assert_eq!(meta["task_name_2"], json!("a"));
    assert_eq!(outcome.repairs, vec!["`Task Name` → `task_name_2`"]);
}

#[test]
fn keys_inside_a_metadata_value_are_data() {
    let doc = r#"[{"title":"t","status":"open","metadata":{"my_key":{"Inner Key":"v"}}}]"#;
    let outcome = read_tasks(doc, NOW).unwrap();
    assert!(outcome.repairs.is_empty());
    assert_eq!(outcome.tasks[0].metadata["my_key"]["Inner Key"], json!("v"));
}

#[test]
fn a_document_with_only_valid_keys_is_returned_unchanged() {
    let doc = r#"{"title":"t","status":"open","metadata":{"due_date":"2026-01-01"}}"#;
    let (out, repairs) = super::repair::repair(Format::Json, doc).unwrap();
    assert_eq!(out, doc);
    assert!(repairs.is_empty());
}

#[test]
fn a_foreign_csv_header_is_repaired() {
    let text = "title,status, Task Name , Due-Date \nBuy milk,open,urgent,2026-01-01\n";
    let outcome = read_tasks(text, NOW).unwrap();
    assert_eq!(outcome.tasks[0].due_date.as_deref(), Some("2026-01-01"));
    assert_eq!(outcome.tasks[0].metadata["task_name"], json!("urgent"));
    assert_eq!(outcome.repairs.len(), 2);
}

#[test]
fn a_slug_that_hits_a_reserved_column_keeps_the_metadata_prefix() {
    let text = "title,status,Metadata\nTask,open,note\n";
    let outcome = read_tasks(text, NOW).unwrap();
    assert_eq!(outcome.tasks[0].metadata["metadata"], json!("note"));
    assert_eq!(outcome.repairs, vec!["`Metadata` → `metadata`"]);
}

#[test]
fn an_invalid_document_reports_the_library_error() {
    let err = read_tasks(r#"[{"title":"t"}]"#, NOW).unwrap_err();
    assert!(err.contains("status"), "{err}");
}

#[test]
fn markdown_is_not_an_export_format() {
    assert!(write_tasks("markdown", &[]).is_err());
}

#[test]
fn the_summary_counts_what_the_import_did() {
    let result = ImportResult {
        imported: 3,
        updated: 1,
        skipped: 0,
        errors: Vec::new(),
        repairs: vec!["`Task Name` → `task_name`".into()],
    };
    assert_eq!(result.summary(), "imported 3, updated 1, skipped 0");
    assert!(!result.is_empty());
    assert!(ImportResult::default().is_empty());
}
