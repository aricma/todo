//! Finding the files worth showing, from a directory or an explicit path.
//!
//! Bounded by [`crate::limits`]: the walk stops at a depth, a number of files and
//! a file size, and says so instead of quietly showing less than the tree holds.

use std::fs;
use std::path::{Path, PathBuf};

use super::{is_candidate, order, Entry};
use crate::doc::Doc;
use crate::error::Error;
use crate::limits;

/// Directories never descended into, whatever they hold.
const SKIPPED: [&str; 4] = ["node_modules", "target", "tmp", ".git"];

/// Which files a walk looks at.
///
/// The convention — `TODO*`, `TASK*`, `README.md` — is what a team that follows it
/// wants; a team that does not has todos in files called anything, and asking for
/// all of them is a deliberate act, because a large tree has a lot of markdown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// Only the files named for todos.
    Named,
    /// Every `*.md` in the tree.
    EveryMarkdown,
}

impl Scope {
    /// Whether a file of this name is worth reading.
    fn wants(self, name: &str) -> bool {
        match self {
            Self::Named => is_candidate(name),
            Self::EveryMarkdown => name.to_lowercase().ends_with(".md"),
        }
    }
}

/// What a scan found, and what it left alone.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scan {
    /// The candidate files, sorted by path.
    pub entries: Vec<Entry>,
    /// One line per thing the limits stopped, for the status line and `--check`.
    pub warnings: Vec<String>,
}

/// The files to show, from a directory or an explicit file.
///
/// A directory yields candidate files that hold at least one task; an explicit
/// file is shown whatever it holds. A non-UTF-8 file is an error as an argument
/// and skipped in a walk.
pub fn scan(path: &Path) -> Result<Scan, Error> {
    scan_scope(path, Scope::Named)
}

/// The same, over the files `scope` names.
pub fn scan_scope(path: &Path, scope: Scope) -> Result<Scan, Error> {
    if !path.exists() {
        return Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("no such file or directory: {}", path.display()),
        )));
    }
    let mut scan = Scan::default();
    if path.is_file() {
        let (entry, _) = entry(path)?;
        scan.entries.push(entry);
        return Ok(scan);
    }
    let mut deeper = Vec::new();
    walk(path, 0, scope, &mut scan, &mut deeper);
    // what a file viewer shows: folders first, then files, without regard to case
    scan.entries
        .sort_by(|left, right| order::compare(&left.path, &right.path));
    // one line per kind of limit, not one per folder: a vendored tree has dozens
    // of folders at the depth limit and none of them holds a todo
    if let Some(first) = deeper.first() {
        scan.warnings.push(format!(
            "[warn] {} deeper than {} levels, not searched (first: {})",
            match deeper.len() {
                1 => "1 folder is".to_owned(),
                count => format!("{count} folders are"),
            },
            limits::MAX_DEPTH,
            first.display()
        ));
    }
    Ok(scan)
}

fn walk(directory: &Path, depth: usize, scope: Scope, scan: &mut Scan, deeper: &mut Vec<PathBuf>) {
    if scan.entries.len() >= limits::MAX_FILES {
        return;
    }
    let Ok(read) = fs::read_dir(directory) else {
        return;
    };
    for item in read.flatten() {
        if scan.entries.len() >= limits::MAX_FILES {
            scan.warnings.push(format!(
                "[warn] {} files is the limit; {} was not searched",
                limits::MAX_FILES,
                directory.display()
            ));
            return;
        }
        let path = item.path();
        let name = item.file_name().to_string_lossy().to_lowercase();
        if name.starts_with('.') || SKIPPED.contains(&name.as_str()) {
            continue;
        }
        if path.is_dir() {
            if depth < limits::MAX_DEPTH {
                walk(&path, depth + 1, scope, scan, deeper);
            } else {
                deeper.push(path);
            }
        } else if scope.wants(&name) {
            if let Ok((entry, worth_showing)) = entry(&path) {
                if worth_showing {
                    scan.entries.push(entry);
                }
            }
        }
    }
}

/// The entry, and whether the file is worth listing: it must hold a task, a line
/// that meant to be one, or nothing but headings — a file made for the first todo
/// is a place to go, and one that holds only prose is not in the business of todos.
fn entry(path: &Path) -> Result<(Entry, bool), Error> {
    let bytes = fs::read(path)?;
    let text = String::from_utf8(bytes).map_err(|_| Error::NotUtf8(PathBuf::from(path)))?;
    let doc = Doc::parse(&text);
    let stats = doc.stats();
    let too_long = doc.lines().len() > limits::MAX_LINES;
    let bare = text
        .lines()
        .all(|line| line.trim().is_empty() || line.trim_start().starts_with('#'));
    let worth_showing = stats.all > 0 || !doc.findings().is_empty() || bare;
    Ok((
        Entry {
            path: PathBuf::from(path),
            stats,
            too_long,
            collections: doc.collections(),
        },
        worth_showing,
    ))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::scan;
    use crate::limits;

    fn write(root: &Path, relative: &str, text: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn names(root: &Path) -> Vec<String> {
        scan(root)
            .unwrap()
            .entries
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
        write(root, "README.md", "# nothing to do\n\nJust a readme, no tasks in it.\n");
        write(root, "notes.md", "- [ ] not a candidate\n");
        write(root, "node_modules/x/TODOs.md", "- [ ] hidden\n");
        write(root, "target/TODOs.md", "- [ ] hidden\n");
        write(root, ".hidden/TODOs.md", "- [ ] hidden\n");
        // folders first, then the files at the root — the file viewer's order
        assert_eq!(names(root), ["sub/TASKS.md", "TODOs.md"]);
    }

    #[test]
    fn a_file_with_nothing_but_headings_is_listed_and_one_of_prose_is_not() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        // what `n` makes, and what a `touch TODOs.md` makes: places to write the
        // first todo, so they have to be reachable from the list
        write(root, "made/TODOs.md", "# made todos\n");
        write(root, "touched/TODOs.md", "");
        // a readme with words in it is not about todos
        write(root, "README.md", "# readme\n\nHow to build it.\n");

        assert_eq!(names(root), ["made/TODOs.md", "touched/TODOs.md"]);
    }

    #[test]
    fn an_explicit_file_is_shown_even_without_tasks() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("README.md");
        fs::write(&path, "# nothing\n").unwrap();
        let scan = scan(&path).unwrap();
        assert_eq!(scan.entries.len(), 1);
        assert_eq!(scan.entries[0].stats, crate::model::Stats::default());
        assert_eq!(scan.entries[0].path, path);
    }

    #[test]
    fn the_walk_stops_at_the_depth_limit_and_says_where() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let deep: String = (0..=limits::MAX_DEPTH).map(|_| "d/").collect();
        write(root, &format!("{deep}TODOs.md"), "- [ ] too deep\n");
        let shallow: String = (0..limits::MAX_DEPTH).map(|_| "d/").collect();
        write(root, &format!("{shallow}TODOs.md"), "- [ ] at the limit\n");

        let scan = scan(root).unwrap();
        assert_eq!(scan.entries.len(), 1, "{:?}", scan.entries);
        assert_eq!(scan.warnings.len(), 1, "{:?}", scan.warnings);
        assert!(
            scan.warnings[0].starts_with("[warn] "),
            "a level, the way a log line has one: {:?}",
            scan.warnings
        );
        assert!(
            scan.warnings[0].contains("deeper than 10 levels"),
            "{:?}",
            scan.warnings
        );
        assert!(
            scan.warnings[0].contains("not searched"),
            "{:?}",
            scan.warnings
        );
    }

    #[test]
    fn a_file_over_the_line_limit_is_listed_but_marked() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let long: String = std::iter::repeat_n("- [ ] a line\n", limits::MAX_LINES + 1).collect();
        write(root, "TODOs.md", &long);
        write(root, "TASKS.md", "- [ ] short\n");

        let scan = scan(root).unwrap();
        assert_eq!(scan.entries.len(), 2);
        let too_long = |name: &str| {
            scan.entries
                .iter()
                .find(|entry| entry.path.ends_with(name))
                .unwrap()
                .too_long
        };
        assert!(too_long("TODOs.md"), "the long one is marked");
        assert!(!too_long("TASKS.md"), "the short one is not");
    }

    #[test]
    fn the_file_limit_stops_the_walk_with_a_warning() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        for index in 0..=limits::MAX_FILES {
            write(root, &format!("d{index}/TODOs.md"), "- [ ] a\n");
        }
        let scan = scan(root).unwrap();
        assert_eq!(scan.entries.len(), limits::MAX_FILES);
        assert!(!scan.warnings.is_empty(), "{:?}", scan.warnings);
    }

    #[test]
    fn stats_travel_with_the_file() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(
            root,
            "TODOs.md",
            "- [ ] a #home #home @sam\n- [x] b #work @sam\n- [x] c\n",
        );
        assert_eq!(
            scan(root).unwrap().entries[0].stats,
            crate::model::Stats {
                all: 3,
                done: 2,
                // counted once however often they are written
                tags: 2,
                assignees: 1,
            }
        );
    }

    #[test]
    fn a_non_utf8_file_is_skipped_in_a_directory_and_an_error_as_an_argument() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let path = root.join("TODOs.md");
        fs::write(&path, [0xff, 0xfe, 0x00]).unwrap();
        assert!(scan(root).unwrap().entries.is_empty());
        assert!(matches!(scan(&path), Err(crate::error::Error::NotUtf8(_))));
    }

    #[test]
    fn a_missing_path_is_an_error() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(matches!(
            scan(&tmp.path().join("nope")),
            Err(crate::error::Error::Io(_))
        ));
    }
}

#[cfg(test)]
mod scope_tests {
    use std::fs;

    use super::{scan, scan_scope, Scope};

    #[test]
    fn every_markdown_file_is_a_candidate_when_the_scope_says_so() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("TODOs.md"), "- [ ] a\n").unwrap();
        fs::write(tmp.path().join("backlog.md"), "- [ ] b\n").unwrap();
        fs::write(tmp.path().join("notes.txt"), "- [ ] c\n").unwrap();

        let named = scan(tmp.path()).unwrap();
        let all = scan_scope(tmp.path(), Scope::EveryMarkdown).unwrap();
        let seen = |found: &super::Scan| -> Vec<String> {
            found
                .entries
                .iter()
                .map(|entry| {
                    entry
                        .path
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .into_owned()
                })
                .collect()
        };
        assert_eq!(seen(&named), ["TODOs.md"]);
        // `backlog.md` too, and still not the text file: a todo needs a markdown file
        assert_eq!(seen(&all), ["backlog.md", "TODOs.md"]);
    }
}
