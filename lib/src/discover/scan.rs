//! Finding the files worth showing, from a directory or an explicit path.
//!
//! Bounded by [`crate::limits`]: the walk stops at a depth, a number of files and
//! a file size, and says so instead of quietly showing less than the tree holds.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{is_candidate, order, Entry};
use crate::doc::{Doc, Source};
use crate::model::Stats;
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

/// An entry for a file that is not read: named so a caller can still point at it.
fn blank(path: &Path, source: Source) -> Entry {
    Entry {
        path: PathBuf::from(path),
        source,
        stats: Stats::default(),
        too_long: false,
        collections: Vec::new(),
    }
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
        let (entry, _) = entry(path, true)?;
        scan.entries.push(entry);
        return Ok(scan);
    }
    // git already knows the tree, and it knows what the team decided to leave out:
    // asking it is what keeps `node_modules`, `dist` and a vendored copy from being
    // read, counted and shown as todos (RFC-017). Without git the walk still runs,
    // with the folders every project agrees on.
    let mut deeper = Vec::new();
    match included(path) {
        Some(files) => walk_listed(path, &files, scope, &mut scan, &mut deeper),
        None => walk(path, 0, scope, &mut scan, &mut deeper, &mut 0),
    }
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

/// The files git does not ignore, as absolute paths, or `None` when this is not a
/// git work tree, git is not installed, or git cannot read it.
///
/// `--cached --others --exclude-standard` is what "the tree" means: what is tracked,
/// plus what is new and not ignored. Ignored paths never appear, so an ignored folder
/// is never descended into.
fn included(root: &Path) -> Option<Vec<PathBuf>> {
    let inside = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .ok()?;
    if !inside.status.success() || inside.stdout != b"true\n" {
        return None;
    }
    let listed = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
            "--",
            ".",
        ])
        .output()
        .ok()?;
    if !listed.status.success() {
        return None;
    }
    let files = String::from_utf8_lossy(&listed.stdout)
        .split('\0')
        .filter(|name| !name.is_empty())
        .map(|name| root.join(name))
        .collect();
    Some(files)
}

/// Walk the list git gave us, keeping the same limits the directory walk keeps.
fn walk_listed(
    root: &Path,
    files: &[PathBuf],
    scope: Scope,
    scan: &mut Scan,
    deeper: &mut Vec<PathBuf>,
) {
    let mut seen = 0;
    for path in files {
        if seen >= limits::MAX_READ {
            scan.warnings.push(format!(
                "[warn] {} files is the limit; the walk stopped there",
                limits::MAX_READ
            ));
            return;
        }
        let relative = path.strip_prefix(root).unwrap_or(path);
        if relative.components().count() > limits::MAX_DEPTH {
            deeper.push(path.clone());
            continue;
        }
        if scan.entries.len() >= limits::MAX_FILES {
            scan.warnings.push(format!(
                "[warn] {} files is the limit; {}", limits::MAX_FILES, relative.display()
            ));
            return;
        }
        // the markdown switch: a tree of every `.md` asks for more of them, and a
        // code file is read either way
        let name = relative
            .file_name()
            .map(|name| name.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if Source::of(path) == Source::Document && !scope.wants(&name) {
            continue;
        }
        seen += 1;
        // the folders a git repository always holds and never lists
        if relative
            .components()
            .any(|part| SKIPPED.contains(&part.as_os_str().to_string_lossy().to_lowercase().as_str()))
        {
            continue;
        }
        if let Ok((entry, worth_showing)) = entry(path, false) {
            if worth_showing {
                scan.entries.push(entry);
            }
        }
    }
}

fn walk(
    directory: &Path,
    depth: usize,
    scope: Scope,
    scan: &mut Scan,
    deeper: &mut Vec<PathBuf>,
    seen: &mut usize,
) {
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
                walk(&path, depth + 1, scope, scan, deeper, seen);
            } else {
                deeper.push(path);
            }
        } else if scope.wants(&name) || Source::of(&path) == Source::Comment {
            if *seen >= limits::MAX_READ {
                scan.warnings.push(format!(
                    "[warn] {} files is the limit; the walk stopped there",
                    limits::MAX_READ
                ));
                return;
            }
            *seen += 1;
            if let Ok((entry, worth_showing)) = entry(&path, false) {
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
fn entry(path: &Path, strict: bool) -> Result<(Entry, bool), Error> {
    let source = Source::of(path);
    // a big file is not read at all: no todo document and no code file worth its
    // size in waiting, and the cap is what keeps a walk through a repository short
    let size = fs::metadata(path).map_or(0, |meta| meta.len());
    if size > limits::MAX_BYTES {
        return Ok((
            Entry {
                too_long: true,
                ..blank(path, source)
            },
            false,
        ));
    }
    let bytes = fs::read(path)?;
    // in a walk, a binary file holds no comments worth reading and a NUL byte is how
    // a text file says it is not one; named on the command line, it is the error the
    // reader asked about
    let binary = !strict && bytes.contains(&0);
    let text = match String::from_utf8(bytes) {
        Ok(text) if !binary => text,
        Ok(_) => return Ok((blank(path, source), false)),
        Err(_) if !strict => return Ok((blank(path, source), false)),
        Err(_) => return Err(Error::NotUtf8(PathBuf::from(path))),
    };
    let doc = match source {
        Source::Document => Doc::parse(&text),
        Source::Comment => Doc::parse_comment(&text),
    };
    let stats = doc.stats();
    let too_long = doc.lines().len() > limits::MAX_LINES;
    let bare = text
        .lines()
        .all(|line| line.trim().is_empty() || line.trim_start().starts_with('#'));
    // a todo document is worth listing when it holds a task, a finding or nothing but
    // headings; a code file only when it holds a `TODO:` comment — the rest of the
    // tree is code, and a file list is a list of places to go, not of files
    let worth_showing = match source {
        Source::Document => stats.all > 0 || !doc.findings().is_empty() || bare,
        Source::Comment => stats.all > 0,
    };
    Ok((
        Entry {
            path: PathBuf::from(path),
            source,
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
    fn a_code_file_is_listed_when_it_holds_a_todo_comment() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(root, "src/main.rs", "fn main() {\n    // TODO: wire it up\n}\n");
        write(root, "src/quiet.rs", "fn main() {}\n");
        write(root, "README.md", "# plain\n\nsome words, no todos\n");
        let scan = scan(root).unwrap();
        let names: Vec<_> = scan
            .entries
            .iter()
            .map(|entry| {
                (
                    entry.path.strip_prefix(root).unwrap().to_string_lossy().into_owned(),
                    entry.source,
                )
            })
            .collect();
        assert_eq!(
            names,
            [("src/main.rs".to_owned(), crate::doc::Source::Comment)],
            "{names:?}"
        );
    }

    #[test]
    fn a_binary_file_is_not_read_for_comments() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::write(root.join("logo.png"), [0x89, b'P', b'N', b'G', 0, 0, b'T', b'O'])
            .unwrap();
        assert!(scan(root).unwrap().entries.is_empty());
    }

    #[test]
    fn what_git_ignores_is_not_read_at_all() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        if std::process::Command::new("git")
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        write(root, ".gitignore", "build/\n");
        write(root, "src/a.rs", "// TODO: kept\n");
        write(root, "build/b.rs", "// TODO: ignored by git\n");
        write(root, "dist/c.rs", "// TODO: not ignored, listed\n");
        let init = std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["init", "-q"])
            .output()
            .unwrap();
        assert!(init.status.success());
        write(root, "dist/.keep", "");
        let names: Vec<String> = scan(root)
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
            .collect();
        assert_eq!(
            names,
            ["dist/c.rs".to_owned(), "src/a.rs".to_owned()],
            "{names:?}"
        );
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
