//! The todos of one collection, read from the files that carry it.

use crate::discover::Entry;
use crate::doc::Doc;
use crate::error::Error;
use crate::model::{Collection, Item, Kind};
use crate::store;

/// Every todo carrying this collection, in file order: files as the scan sorted
/// them, and inside a file the order the todos are written in.
///
/// The order is the point — a collection is read as "what do I owe about this",
/// and keeping a file's own sequence keeps the reading order someone chose. A file
/// that cannot be read fails the whole gather: half a collection that looks whole is
/// worse than a message saying why.
pub fn gather(entries: &[Entry], collection: &Collection) -> Result<Vec<Item>, Error> {
    let mut items = Vec::new();
    for entry in entries {
        // only the files that actually mention it: the scan already said which
        if !entry
            .collections
            .iter()
            .any(|found| found.kind == collection.kind && found.name == collection.name)
        {
            continue;
        }
        let loaded = store::load(&entry.path)?;
        let doc = Doc::parse(&loaded.text);
        for task in doc.tasks() {
            let carries = match collection.kind {
                Kind::Tag => task.tags.iter().any(|tag| tag.as_str() == collection.name),
                Kind::Assignee => task
                    .assignees
                    .iter()
                    .any(|who| who.as_str() == collection.name),
            };
            if carries {
                items.push(Item {
                    path: entry.path.clone(),
                    line: task.line,
                    lead: task.lead(),
                    body: task.body.clone(),
                    done: task.done,
                });
            }
        }
    }
    Ok(items)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::gather;
    use crate::discover::scan;
    use crate::model::{Collection, Kind};

    #[test]
    fn gathers_in_file_order_across_files() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("b")).unwrap();
        fs::write(
            root.join("TODOs.md"),
            "- [ ] one #home\n- [ ] two\n- [x] three #home\n",
        )
        .unwrap();
        fs::write(root.join("b/TODOs.md"), "- [ ] four #home\n").unwrap();

        let entries = scan(root).unwrap().entries;
        let items = gather(
            &entries,
            &Collection {
                kind: Kind::Tag,
                name: "home".to_owned(),
                count: 3,
                done: 0,
            },
        )
        .unwrap();
        // the files as the scan sorted them — folders before files — and inside
        // each file its own order
        let bodies: Vec<&str> = items.iter().map(|item| item.body.as_str()).collect();
        assert_eq!(bodies, ["four #home", "one #home", "three #home"]);
        assert!(items[0].path.ends_with("b/TODOs.md"));
        assert_eq!(items[2].line, 2, "the line is the anchor a write needs");
        assert!(
            items[2].done,
            "done comes with it, so a collection can count"
        );
    }

    #[test]
    fn a_collection_with_nothing_in_it_gathers_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("TODOs.md"), "- [ ] one\n").unwrap();
        let entries = scan(tmp.path()).unwrap().entries;
        let items = gather(
            &entries,
            &Collection {
                kind: Kind::Assignee,
                name: "sam".to_owned(),
                count: 0,
                done: 0,
            },
        )
        .unwrap();
        assert!(items.is_empty());
    }
}
