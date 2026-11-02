//! Finding the files worth showing, from a directory or an explicit path.

use std::fs;
use std::path::{Path, PathBuf};

use super::{is_candidate, Entry};
use crate::doc::Doc;
use crate::error::Error;

/// How deep the directory walk goes below the path it was given.
const MAX_DEPTH: usize = 6;

/// Directories never descended into, whatever they hold.
const SKIPPED: [&str; 4] = ["node_modules", "target", "tmp", ".git"];

/// The files to show, sorted by path.
///
/// A directory yields candidate files that hold at least one task; an explicit
/// file is shown whatever it holds, and a non-UTF-8 file is an error there and
/// skipped here.
pub fn scan(path: &Path) -> Result<Vec<Entry>, Error> {
    if !path.exists() {
        return Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("no such file or directory: {}", path.display()),
        )));
    }
    if path.is_file() {
        return Ok(vec![entry(path)?.0]);
    }
    let mut entries = Vec::new();
    walk(path, 0, &mut entries);
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(entries)
}

fn walk(directory: &Path, depth: usize, entries: &mut Vec<Entry>) {
    let Ok(read) = fs::read_dir(directory) else {
        return;
    };
    for item in read.flatten() {
        let path = item.path();
        let name = item.file_name().to_string_lossy().to_lowercase();
        if name.starts_with('.') || SKIPPED.contains(&name.as_str()) {
            continue;
        }
        if path.is_dir() {
            if depth < MAX_DEPTH {
                walk(&path, depth + 1, entries);
            }
        } else if is_candidate(&name) {
            if let Ok((entry, interesting)) = entry(&path) {
                if interesting {
                    entries.push(entry);
                }
            }
        }
    }
}

/// The entry, and whether the file is worth showing: it must hold a task, or at
/// least a line that meant to be one — otherwise a file of nothing but broken
/// todos would be invisible to `--check`.
fn entry(path: &Path) -> Result<(Entry, bool), Error> {
    let bytes = fs::read(path)?;
    let text = String::from_utf8(bytes).map_err(|_| Error::NotUtf8(PathBuf::from(path)))?;
    let doc = Doc::parse(&text);
    let stats = doc.stats();
    let interesting = stats.all > 0 || !doc.findings().is_empty();
    Ok((
        Entry {
            path: PathBuf::from(path),
            stats,
        },
        interesting,
    ))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::scan;
    use crate::error::Error;
    use crate::model::Stats;

    fn write(root: &Path, relative: &str, text: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn names(root: &Path) -> Vec<String> {
        scan(root)
            .unwrap()
            .into_iter()
            .map(|entry| {
                entry
                    .path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect()
    }

    #[test]
    fn a_directory_yields_todo_files_with_tasks_only() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(root, "TODOs.md", "- [ ] a\n");
        write(root, "sub/TASKS.md", "- [x] b\n");
        write(root, "README.md", "# nothing to do\n");
        write(root, "notes.md", "- [ ] not a candidate\n");
        write(root, "node_modules/x/TODOs.md", "- [ ] hidden\n");
        write(root, "target/TODOs.md", "- [ ] hidden\n");
        write(root, ".hidden/TODOs.md", "- [ ] hidden\n");
        assert_eq!(names(root), ["TODOs.md", "sub/TASKS.md"]);
    }

    #[test]
    fn an_explicit_file_is_shown_even_without_tasks() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("README.md");
        fs::write(&path, "# nothing\n").unwrap();
        let entries = scan(&path).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, path);
        assert_eq!(entries[0].stats, Stats::default());
    }

    #[test]
    fn nested_directories_are_walked_to_depth_six() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(root, "a/b/c/d/e/f/TODOs.md", "- [ ] six\n");
        write(root, "a/b/c/d/e/f/g/TODOs.md", "- [ ] seven\n");
        assert_eq!(names(root), ["a/b/c/d/e/f/TODOs.md"]);
    }

    #[test]
    fn results_are_sorted_by_path() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(root, "b/TODOs.md", "- [ ] b\n");
        write(root, "TODOs.md", "- [ ] root\n");
        write(root, "a/TODOs.md", "- [ ] a\n");
        assert_eq!(names(root), ["TODOs.md", "a/TODOs.md", "b/TODOs.md"]);
    }

    #[test]
    fn a_file_with_only_a_broken_task_is_still_listed() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "TODOs.md", "- [ ]no space\n");
        assert_eq!(names(tmp.path()), ["TODOs.md"]);
        assert_eq!(scan(tmp.path()).unwrap()[0].stats.all, 0);
    }

    #[test]
    fn stats_travel_with_the_file() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(root, "TODOs.md", "- [ ] a\n- [x] b\n- [x] c\n");
        assert_eq!(scan(root).unwrap()[0].stats, Stats { all: 3, done: 2 });
    }

    #[test]
    fn a_non_utf8_file_is_skipped_in_a_directory_and_an_error_as_an_argument() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("TODOs.md");
        fs::write(&path, [0xff, 0xfe, 0x00]).unwrap();
        assert!(scan(root).unwrap().is_empty());
        assert!(matches!(scan(&path), Err(Error::NotUtf8(_))));
    }

    #[test]
    fn a_missing_path_is_an_error() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nope");
        assert!(matches!(scan(&path), Err(Error::Io(_))));
    }
}
