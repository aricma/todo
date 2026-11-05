//! Searching every todo of every file the scan found.

use super::fuzzy;
use super::Hit;
use crate::discover::Entry;
use crate::doc::Doc;
use crate::error::Error;
use crate::store;

/// Every todo matching `query`, best first, then by file and line.
///
/// Only todos are searched — not headings, not prose — because the result is
/// something to jump to and act on.
pub fn find(files: &[Entry], query: &str) -> Result<Vec<Hit>, Error> {
    let mut hits = Vec::new();
    for file in files.iter().filter(|entry| !entry.too_long) {
        let loaded = store::load(&file.path)?;
        for task in Doc::parse_as(file.source, &loaded.text).tasks() {
            if let Some(score) = fuzzy::score(query, &task.body) {
                hits.push(Hit {
                    path: file.path.clone(),
                    line: task.line,
                    body: task.body,
                    score,
                });
            }
        }
    }
    hits.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line.cmp(&right.line))
    });
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::find;
    use crate::discover::scan;

    fn files(root: &Path, sources: &[(&str, &str)]) -> Vec<crate::discover::Entry> {
        for (name, text) in sources {
            let path = root.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        scan(root).unwrap().entries
    }

    #[test]
    fn only_todos_are_found_and_the_best_match_comes_first() {
        let tmp = tempfile::tempdir().unwrap();
        let files = files(
            tmp.path(),
            &[(
                "TODOs.md",
                "# milk\n\n- [ ] email the landlord\n- [ ] buy milk\n",
            )],
        );
        // "ml": a word-start match in "buy milk" beats the letters scattered
        // through "email"; the heading "milk" is not a result at all
        let hits = find(&files, "ml").unwrap();
        let bodies: Vec<&str> = hits.iter().map(|hit| hit.body.as_str()).collect();
        assert_eq!(bodies, ["buy milk", "email the landlord"]);
    }

    #[test]
    fn a_hit_carries_the_file_and_line_to_jump_to() {
        let tmp = tempfile::tempdir().unwrap();
        let files = files(tmp.path(), &[("TODOs.md", "- [ ] buy milk\n")]);
        let hits = find(&files, "milk").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].line, 0);
        assert!(hits[0].path.ends_with("TODOs.md"));
    }

    #[test]
    fn an_empty_query_lists_every_todo() {
        let tmp = tempfile::tempdir().unwrap();
        let files = files(
            tmp.path(),
            &[
                ("TODOs.md", "- [ ] a\n- [x] b\n"),
                ("TASKS.md", "- [ ] c\n"),
            ],
        );
        let hits = find(&files, "").unwrap();
        assert_eq!(hits.len(), 3);
    }

    #[test]
    fn a_file_over_the_line_limit_is_not_searched() {
        let tmp = tempfile::tempdir().unwrap();
        let long: String =
            std::iter::repeat_n("- [ ] needle\n", crate::limits::MAX_LINES + 1).collect();
        let files = files(tmp.path(), &[("TODOs.md", &long)]);
        assert!(files[0].too_long);
        assert!(find(&files, "needle").unwrap().is_empty());
    }
}
