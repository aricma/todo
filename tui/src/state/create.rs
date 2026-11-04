//! Creating a todo file: the folder typed, what that folder offers, and why the
//! path is refused.
//!
//! Only the folder is typed. The file always gets the one name the scan looks for,
//! so nobody has to remember which names count.

use std::path::{Component, Path, PathBuf};

use super::prompt::Prompt;
use crate::disk;
use crate::helpers::split_path;
use crate::text;

/// The name a file created here always gets.
pub const FILE_NAME: &str = "TODOs.md";

/// How many folders are offered at once: enough to pick from, few enough that the
/// field stays a field.
const OFFERS: usize = 5;

/// The whole state of the new-file field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Create {
    pub prompt: Prompt,
    /// Folders that continue what was typed, as paths of the same shape.
    pub suggestions: Vec<String>,
    /// The one the arrows marked, if any.
    pub selected: Option<usize>,
    /// Why `Enter` is blocked, shown above the controls.
    pub refusal: Option<String>,
    /// What the typed path is resolved against.
    root: PathBuf,
}

impl Create {
    /// `start` is the folder the field opens on, as it is typed: `./web` or `.`.
    pub fn new(root: &Path, start: &str) -> Self {
        let mut create = Self {
            prompt: Prompt::new(start),
            suggestions: Vec::new(),
            selected: None,
            refusal: None,
            root: root.to_path_buf(),
        };
        create.refresh();
        create
    }

    /// The typed path, as a folder under the root.
    pub fn folder(&self) -> PathBuf {
        self.root.join(self.prompt.text().trim())
    }

    /// Where the file will be made.
    pub fn file(&self) -> PathBuf {
        self.folder().join(FILE_NAME)
    }

    /// The typed path as it is joined to the root, for `create_dir_all`.
    pub fn typed(&self) -> &str {
        self.prompt.text().trim()
    }

    /// Work out what the folder offers and whether it may be used. Cheap enough to
    /// run on every keystroke, and always right after work is typed.
    pub fn refresh(&mut self) {
        self.selected = None;
        self.suggestions = self.offers();
        self.refusal = self.refuse();
    }

    /// Mark a suggestion, `⬆`/`⬇` among them.
    pub fn mark(&mut self, delta: isize) {
        let last = self.suggestions.len().saturating_sub(1);
        if self.suggestions.is_empty() {
            return;
        }
        self.selected = Some(match (self.selected, delta.is_negative()) {
            (None, true) => last,
            (None, false) => 0,
            (Some(at), true) => at.saturating_sub(delta.unsigned_abs()),
            (Some(at), false) => (at + delta.unsigned_abs()).min(last),
        });
    }

    /// Type one character. A marked suggestion is taken first, so marking one and
    /// typing on is a single motion instead of two.
    pub fn accept(&mut self, character: char) {
        if let Some(chosen) = self.selected.and_then(|index| self.suggestions.get(index)) {
            let chosen = chosen.clone();
            self.prompt = Prompt::new(&chosen);
        }
        self.prompt.insert(character);
        self.refresh();
    }

    /// Move, erase or jump: all of it ends with the offers and the refusal
    /// re-derived, because every one of them changes the path.
    pub fn prompt_mut(&mut self) -> &mut Prompt {
        &mut self.prompt
    }

    /// The folders one level under what was typed, as far as they match it.
    fn offers(&self) -> Vec<String> {
        let (folder, typed) = split(self.typed(), &self.root);
        disk::folders_in(&self.root.join(&folder))
            .into_iter()
            .filter(|name| name.starts_with(&typed))
            .take(OFFERS)
            .map(|name| format!("{folder}{name}"))
            .collect()
    }

    /// Why this path cannot be used, if it cannot.
    fn refuse(&self) -> Option<String> {
        // nothing typed is the root itself: the field shows `./TODOs.md`, so that
        // is a path a person can see and read before pressing enter
        if Path::new(self.typed())
            .components()
            .any(|part| part == Component::ParentDir)
        {
            return Some(text::keep_inside(&self.root.display().to_string()));
        }
        self.file().exists().then(|| text::already_there(FILE_NAME))
    }
}

/// The folder and the part of the name typed after it: `web/sr` is `web/` and
/// `sr`, so an offer is `folder + name`.
///
/// A path that already *is* a folder has no name being typed, so `web` offers what
/// is inside it exactly like `web/` does.
fn split(text: &str, root: &Path) -> (String, String) {
    if text.is_empty() {
        return (String::new(), String::new());
    }
    if !text.ends_with('/') && root.join(text).is_dir() {
        return (format!("{text}/"), String::new());
    }
    split_path(text)
}

#[cfg(test)]
mod tests {
    use super::{split, Create, FILE_NAME};

    fn tree() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        for folder in ["src", "tmp", "tests"] {
            std::fs::create_dir_all(tmp.path().join(folder)).unwrap();
        }
        std::fs::create_dir_all(tmp.path().join("src/inner")).unwrap();
        std::fs::write(tmp.path().join("file.md"), "").unwrap();
        std::fs::create_dir_all(tmp.path().join(".hidden")).unwrap();
        tmp
    }

    #[test]
    fn a_path_splits_into_its_folder_and_the_part_being_typed() {
        let root = std::path::Path::new("/nowhere");
        assert_eq!(
            split("./web/sr", root),
            ("./web/".to_owned(), "sr".to_owned())
        );
        assert_eq!(split("./web/", root), ("./web/".to_owned(), String::new()));
        assert_eq!(split("src", root), (String::new(), "src".to_owned()));
        assert_eq!(split("", root), (String::new(), String::new()));
    }

    #[test]
    fn only_folders_that_match_are_offered_and_never_hidden_ones() {
        let tmp = tree();
        let create = Create::new(tmp.path(), "");
        let offers = create.suggestions;
        assert_eq!(offers, vec!["src", "tests", "tmp"], "{offers:?}");

        let mut typed = Create::new(tmp.path(), "sr");
        assert_eq!(typed.suggestions, vec!["src"]);
        // typing the last letter finishes the folder name, so the offers move
        // inside it rather than repeating what was just typed
        typed.accept('c');
        assert_eq!(typed.typed(), "src");
        assert_eq!(typed.suggestions, vec!["src/inner"]);
        assert_eq!(typed.file(), tmp.path().join("src").join(FILE_NAME));
    }

    #[test]
    fn the_arrows_mark_one_of_the_offers_and_typing_takes_it() {
        let tmp = tree();
        let mut create = Create::new(tmp.path(), "");
        assert_eq!(create.selected, None);
        create.mark(1);
        assert_eq!(create.selected, Some(0));
        create.mark(1);
        assert_eq!(create.selected, Some(1));
        create.mark(-1);
        assert_eq!(create.selected, Some(0));

        // typing on continues from the marked folder
        create.accept('/');
        assert_eq!(create.typed(), "src/");
        assert_eq!(create.selected, None, "the mark was spent");
        assert_eq!(create.suggestions, vec!["src/inner"]);
    }

    #[test]
    fn a_file_that_is_already_there_blocks_enter() {
        let tmp = tree();
        let mut create = Create::new(tmp.path(), "");
        assert_eq!(create.refusal, None);
        create.accept('s');
        create.accept('r');
        create.accept('c');
        assert_eq!(create.typed(), "src");
        // ./src exists but is empty: the file is what matters, so this is allowed
        assert_eq!(create.refusal, None);
        std::fs::write(tmp.path().join("src").join(FILE_NAME), "").unwrap();
        create.refresh();
        assert_eq!(
            create.refusal.as_deref(),
            Some("TODOs.md is already there — nothing is overwritten")
        );
    }

    #[test]
    fn a_path_out_of_the_root_is_refused_and_nothing_typed_is_the_root() {
        let tmp = tree();
        let mut create = Create::new(tmp.path(), "..");
        assert!(create
            .refusal
            .as_deref()
            .is_some_and(|why| why.contains("inside")));
        // asking again about the same path is the same answer
        create.refresh();
        assert!(create.refusal.is_some());

        // nothing typed is `./TODOs.md`: a path the field shows in full, so it is
        // allowed, and it is the root that has no todo file in the fixture
        let root = Create::new(tmp.path(), "");
        assert_eq!(root.refusal, None);
        assert_eq!(root.suggestions.len(), 3, "the root's own folders");
        assert_eq!(
            std::fs::read_dir(tmp.path()).unwrap().count(),
            5,
            "nothing was made"
        );
    }
}
