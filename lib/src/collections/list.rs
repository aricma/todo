//! What collections exist, and how big each one is.

use std::collections::BTreeMap;

use crate::discover::Entry;
use crate::model::{Collection, Kind};

/// Every collection the scanned files mention, with how many todos carry it.
///
/// Tags first, then people, each **biggest first**: a list of forty tags is read to
/// find the busy ones, and a name is easier to find with `f` than by scrolling. Ties
/// fall back to the name, so the rows do not move around between scans. Names are
/// compared exactly: `#Home` and `#home` are two collections, because a tag is a
/// word someone wrote.
pub fn list(entries: &[Entry]) -> Vec<Collection> {
    let mut found: BTreeMap<(Kind, String), (usize, usize)> = BTreeMap::new();
    for entry in entries {
        for collection in &entry.collections {
            let counts = found
                .entry((collection.kind, collection.name.clone()))
                .or_default();
            counts.0 += collection.count;
            counts.1 += collection.done;
        }
    }
    let mut collected: Vec<Collection> = found
        .into_iter()
        .map(|((kind, name), (count, done))| Collection {
            kind,
            name,
            count,
            done,
        })
        .collect();
    // the two kinds stay apart — one list each, in the order the panel shows them —
    // and inside a kind the busiest comes first
    collected.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| right.count.cmp(&left.count))
            .then_with(|| left.name.cmp(&right.name))
    });
    collected
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::list;
    use crate::discover::scan;
    use crate::model::Kind;

    fn seen(entries: &[crate::discover::Entry]) -> Vec<(Kind, String, usize, usize)> {
        list(entries)
            .iter()
            .map(|one| (one.kind, one.name.clone(), one.count, one.done))
            .collect()
    }

    #[test]
    fn sums_a_collection_across_files_and_keeps_the_two_kinds_apart() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("b")).unwrap();
        fs::write(
            root.join("TODOs.md"),
            "- [ ] a #home @sam\n- [x] b #home #home @sam\n",
        )
        .unwrap();
        fs::write(root.join("b/TODOs.md"), "- [ ] c #Home @sam\n").unwrap();

        let found = scan(root).unwrap().entries;
        // tags first then people, and `#Home` is its own collection. Inside a kind
        // the busiest comes first, which is why `#home` is above `#Home` — one is
        // finished, so both carry how much of them is done
        assert_eq!(
            seen(&found),
            [
                (Kind::Tag, "home".to_owned(), 2, 1),
                (Kind::Tag, "Home".to_owned(), 1, 0),
                (Kind::Assignee, "sam".to_owned(), 3, 1),
            ]
        );
    }

    #[test]
    fn the_biggest_collections_come_first_and_a_tie_falls_back_to_the_name() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(
            tmp.path().join("TODOs.md"),
            "- [ ] a #one #two #three\n- [ ] b #two #three\n- [ ] c #three #four\n",
        )
        .unwrap();
        let found = scan(tmp.path()).unwrap().entries;
        let names: Vec<String> = list(&found).iter().map(|one| one.name.clone()).collect();
        // `#three` 3, `#two` 2, then `#four` and `#one` at 1 — a tie by name, so
        // the rows do not swap places between scans
        assert_eq!(names, ["three", "two", "four", "one"]);
    }

    #[test]
    fn a_file_with_no_tags_or_people_adds_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("TODOs.md"), "- [ ] plain\n").unwrap();
        assert!(list(&scan(tmp.path()).unwrap().entries).is_empty());
    }
}
