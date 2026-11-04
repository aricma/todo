//! The order the file list is drawn and walked in.
//!
//! What a file viewer shows: folders first, then files, each in alphanumeric order,
//! so flattening a tree keeps the tree's shape — `crates/lib/TODOs.md` before
//! `README.md`, and `web/Tasks.md` before `web/src/TODOs.md`.

use std::cmp::Ordering;
use std::path::{Component, Path};

/// One level of a path: whether a file is the end of it there, and its name twice —
/// lower case for the comparison, and as written to break a tie between spellings.
type Level = (bool, String, String);

/// Where two files belong relative to each other.
pub fn compare(left: &Path, right: &Path) -> Ordering {
    levels(left).cmp(&levels(right))
}

fn levels(path: &Path) -> Vec<Level> {
    let names: Vec<String> = path
        .components()
        .filter_map(|part| match part {
            Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    let last = names.len().saturating_sub(1);
    names
        .into_iter()
        .enumerate()
        .map(|(index, name)| {
            // a folder sorts before a file at the same level, which is the whole
            // rule: `web/` above `README.md`
            let file = index == last;
            (file, name.to_lowercase(), name)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::compare;

    fn sorted(paths: &[&str]) -> Vec<String> {
        let mut paths: Vec<&Path> = paths.iter().map(Path::new).collect();
        paths.sort_by(|left, right| compare(left, right));
        paths
            .iter()
            .map(|path| path.display().to_string())
            .collect()
    }

    #[test]
    fn folders_come_before_files_at_every_level() {
        assert_eq!(
            sorted(&["README.md", "web/TODOs.md", "crates/TODOs.md"]),
            vec!["crates/TODOs.md", "web/TODOs.md", "README.md"]
        );
    }

    #[test]
    fn everything_is_compared_without_case() {
        assert_eq!(
            sorted(&["TODOs.md", "readme.md", "Tasks.md"]),
            vec!["readme.md", "Tasks.md", "TODOs.md"]
        );
        // and a folder is still a folder whatever its case
        assert_eq!(
            sorted(&["zebra/TODOs.md", "Apple/TODOs.md"]),
            vec!["Apple/TODOs.md", "zebra/TODOs.md"]
        );
    }

    #[test]
    fn a_deeper_path_sorts_after_the_file_it_sits_beside() {
        assert_eq!(
            sorted(&["web/TODOs.md", "web/a/TODOs.md"]),
            vec!["web/a/TODOs.md", "web/TODOs.md"]
        );
    }
}
