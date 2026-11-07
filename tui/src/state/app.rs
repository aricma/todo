//! The TUI's state: three panes, one open file, and the session history.
//!
//! The screen is a function of this: nothing here draws, and nothing here reads
//! the terminal, which is what makes every rule testable without a pty. A key
//! press either changes the state or returns an `Action` for the caller to
//! perform.
//!
//! Files are the one thing it touches, and only in three places: opening one (a
//! `scan` and a `load`), creating or deleting one from the files panel, and the
//! poll that notices an outside write. Every *edit* — the thing a person does all
//! day — leaves as `Action::Save` instead, so the state machine never has to be
//! told how a save went.

use std::path::{Path, PathBuf};

use aricma_todo_lib::collections;
use aricma_todo_lib::discover::{self, Entry as FileEntry};
use aricma_todo_lib::doc::{Doc, Source};
use aricma_todo_lib::error::Error;
use aricma_todo_lib::model::{Collection, Heading, Inverse, Item, Stats, Task};
use aricma_todo_lib::store::{self, Loaded};

use super::action::{Action, Pane};
use super::create::FILE_NAME;
use super::entry::Entry;
use super::message::Message;
use super::prompt::Mode;
use super::row::{Row, Showing};
use super::search::Search;
use crate::disk;
use crate::helpers::shift;
use crate::text;
use ratatui::crossterm::event::KeyEvent;

/// Everything the screen needs, and nothing it does not.
// The four switches share nothing but the struct they live in: `single` is how the
// session was opened, `show_all` and `roomy` are what the reader asked to see, and
// `quit` is a request. A type for each would be a type to satisfy a linter.
#[allow(clippy::struct_excessive_bools)]
pub struct App {
    pub files: Vec<FileEntry>,
    /// The rows of the file panel: the files, then the collections summed from them.
    pub rows: Vec<Row>,
    /// The row the panel's cursor is on. Not the open file: a collection row is
    /// selected without any file being opened.
    pub row_idx: usize,
    /// The collection being read, when one is: its todos, gathered in file order,
    /// each still knowing the file it came from.
    pub collection: Option<Collection>,
    pub items: Vec<Item>,
    /// The file that is open — the document edits go to. With a collection open it
    /// follows the highlighted todo's origin file.
    pub file_idx: usize,
    pub pane: Pane,
    pub task_idx: usize,
    pub outline_idx: usize,
    pub history_idx: usize,
    pub doc: Doc,
    pub baseline: Loaded,
    /// The path the app was opened on: file paths are shown relative to it.
    pub root: PathBuf,
    /// Opened on one file rather than a folder: no file list, no file pane.
    pub single: bool,
    /// Whether the file panel lists every markdown file in the tree, or only the
    /// ones named for todos. `h` is the switch.
    pub show_all: bool,
    /// What the scan's limits stopped, shown in the status bar.
    pub warnings: Vec<String>,
    /// The search popover, closed until `ctrl+f`.
    pub search: Search,
    /// The sidebar section to come back to when leaving the file view.
    pub last_sidebar: Pane,
    pub history: Vec<Entry>,
    pub mode: Mode,
    /// The empty line a new todo is waiting in, while its field is open: the line, and
    /// the row the cursor goes back to if the field is cancelled. The row is in the
    /// document already, so the highlight and the file view point at where the todo
    /// will land — the field is the question, the row is the answer being prepared.
    pub add: Option<(usize, usize)>,
    /// What just happened, drawn under the panel it is about — never in the status
    /// bar. A message about one row lives on that row instead.
    pub message: Option<Message>,
    /// The last key that arrived, as the terminal described it. The keys page shows
    /// it, because "the key does nothing" is usually the terminal, not the app.
    pub last_key: Option<KeyEvent>,
    /// What the file panel lists besides files: `Both` by default, and `c` cycles.
    pub show: Showing,
    /// Whether the rows get a blank one between them. Off by default: a terminal
    /// has no half-line, so "more line height" costs a whole row per item and
    /// halves what fits. `t` turns it on.
    pub roomy: bool,
    /// Whether a history entry is one row with an ellipsis rather than as many rows as
    /// its label needs. On by default: a long write name pushes the rest of the trail
    /// off the panel, and the trail is what the panel is for. `h` widens it.
    pub history_one_line: bool,
    pub quit: bool,
}

impl App {
    /// Scan a path and open its first file.
    pub fn open(path: &Path) -> Result<Self, Error> {
        // `.` is a fine thing to type and a useless thing to read: the header, and
        // every path under it, is the real tree. A path that cannot be resolved is
        // left as it is, so the scan says what is wrong in its own words.
        let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let scan = discover::scan(&path)?;
        if scan.entries.is_empty() {
            return Err(Error::Ops(format!(
                "{}: no todo files found",
                path.display()
            )));
        }
        // a file over the line limit is listed but never read, so opening it by
        // accident is not an option. A todo document wins over a comment todo file: a
        // session opens where the todos were written, not on whichever source file
        // happens to sort first (RFC-015)
        let first = |wanted: Source| {
            scan.entries
                .iter()
                .position(|entry| !entry.too_long && entry.source == wanted)
        };
        let Some(index) = first(Source::Document).or_else(|| first(Source::Comment)) else {
            let blocked = &scan.entries[0];
            return Err(Error::Ops(format!(
                "{}: {}",
                blocked.path.display(),
                blocked.skipped_reason().unwrap_or_default()
            )));
        };
        let single = path.is_file();
        if single {
            // a file named on the command line is shown whatever it holds (the scan
            // says so), but a *session* on one promises there is something to read: a
            // markdown file is a todo document wherever it lives — it is where todos
            // get written — and anything else has to hold at least one todo comment
            // (RFC-015, RFC-016)
            let entry = &scan.entries[index];
            if entry.source == Source::Comment && entry.stats.all == 0 {
                return Err(Error::Ops(format!(
                    "{}: neither a markdown todo file nor a file with a todo comment",
                    entry.path.display()
                )));
            }
        }
        let path_of_entry = &scan.entries[index].path;
        let baseline = store::load(path_of_entry)?;
        let collections = collections::list(&scan.entries);
        let rows = Self::rows_for(&scan.entries, &collections, Showing::Both);
        // the cursor starts on the open file, not on the first row
        let row_idx = index.min(rows.len().saturating_sub(1));
        Ok(Self {
            doc: Doc::parse_at(path_of_entry, &baseline.text),
            files: scan.entries,
            rows,
            row_idx,
            collection: None,
            items: Vec::new(),
            warnings: scan.warnings,
            search: Search::default(),
            file_idx: index,
            task_idx: 0,
            outline_idx: 0,
            history_idx: 0,
            root: path,
            single,
            show_all: false,
            // where `←` goes back to: the sidebar, which in a single-file session is
            // the index and the history
            last_sidebar: if single { Pane::Outline } else { Pane::Files },
            pane: if single { Pane::Todos } else { Pane::Files },
            baseline,
            history: Vec::new(),
            mode: Mode::Normal,
            add: None,
            message: None,
            last_key: None,
            show: Showing::Both,
            roomy: false,
            history_one_line: true,
            quit: false,
        })
    }

    /// The panel's rows: every file, then every collection, so a row only moves
    /// when a file or a name appears.
    fn rows_for(files: &[FileEntry], collections: &[Collection], show: Showing) -> Vec<Row> {
        let mut rows: Vec<Row> = (0..files.len()).map(Row::File).collect();
        rows.extend(
            collections
                .iter()
                .filter(|one| show.wants(one.kind))
                .cloned()
                .map(Row::Collection),
        );
        rows
    }

    /// Show a different half of the panel: files only, then tags, then people.
    pub fn cycle_showing(&mut self) {
        self.show = self.show.next();
        self.rebuild_rows();
    }

    /// Rebuild the rows after a scan, keeping the cursor on the same thing: the same
    /// file, or the same collection, wherever it moved to.
    fn rebuild_rows(&mut self) {
        let collections = collections::list(&self.files);
        let keep = self.selected_row().cloned();
        self.rows = Self::rows_for(&self.files, &collections, self.show);
        self.row_idx = keep
            .and_then(|row| self.rows.iter().position(|candidate| *candidate == row))
            .unwrap_or_else(|| self.row_idx.min(self.rows.len().saturating_sub(1)));
        // a collection whose last todo went away is no longer a row, so it cannot be
        // the one being read either — and whatever the cursor is on is gathered again,
        // because the files it was gathered from have changed. It is the *name* that
        // says which collection this is: the counts change with every write, and a
        // collection that grew is not a different collection
        if let Some(open) = self.collection.clone() {
            let still_a_row = self.rows.iter().any(|row| {
                row.collection()
                    .is_some_and(|one| one.kind == open.kind && one.name == open.name)
            });
            if !still_a_row {
                self.collection = None;
                self.items.clear();
                self.pane = self.last_sidebar;
            }
        }
        self.reload_row();
    }

    /// Gather the collection the cursor is on again, keeping nothing else.
    fn reload_row(&mut self) {
        if let Some(collection) = self.selected_row().and_then(Row::collection).cloned() {
            self.collection = None;
            if let Err(error) = self.load_collection(&collection) {
                self.set_warning(error.to_string());
            }
        }
    }

    pub fn current(&self) -> &FileEntry {
        &self.files[self.file_idx]
    }

    /// The row the panel's cursor is on.
    pub fn selected_row(&self) -> Option<&Row> {
        self.rows.get(self.row_idx)
    }

    /// The file the panel's cursor is on, when it is on a file.
    pub fn selected_file(&self) -> Option<&FileEntry> {
        self.selected_row()?
            .file()
            .and_then(|at| self.files.get(at))
    }

    /// The collection being read, when one is.
    pub fn viewing(&self) -> Option<&Collection> {
        self.collection.as_ref()
    }

    /// The row `delta` lands on, stepping over files over the line limit: they are
    /// listed, dimmed and never opened, so the cursor must not stop on them. A
    /// collection row is never blocked — it is not a file.
    fn reachable(&self, delta: isize) -> Option<usize> {
        let target = shift(self.row_idx, delta, self.rows.len());
        // `g`/`G` jump to an end and walk back from it; a single step walks on
        let step = if delta.unsigned_abs() == 1 {
            delta.signum()
        } else {
            -delta.signum()
        };
        let mut at = target;
        loop {
            match self.rows.get(at)? {
                Row::File(index) if self.files[*index].too_long => {
                    let next = shift(at, step, self.rows.len());
                    if next == at {
                        return None;
                    }
                    at = next;
                }
                _ => return Some(at),
            }
        }
    }

    /// The document's headings, in file order.
    pub fn headings(&self) -> Vec<Heading> {
        self.doc.headings()
    }

    /// The sidebar sections, in the order `Tab` walks them. The file view is
    /// deliberately not one: `→` and `←` are what cross between the two.
    pub fn sections(&self) -> Vec<Pane> {
        let mut sections = Vec::new();
        if !self.single {
            sections.push(Pane::Files);
        }
        // a collection is not a document: it has no headings of its own, and the
        // outline on screen would be the *open file's* — scrolling it would move
        // the collection's cursor through some other file's sections. So it goes
        // away while one is being read, and comes back with the file
        if self.collection.is_none() {
            // always otherwise: an empty outline is where a heading gets made
            sections.push(Pane::Outline);
        }
        sections.push(Pane::History);
        sections
    }

    /// Whether a panel is on screen at all.
    ///
    /// The sidebar's sections are `sections()`; the right-hand column is not a section
    /// — it is the todos panel in a single-file session, a collection while one is open,
    /// the file view otherwise — so a key that jumps by number asks this, not the list.
    pub fn drawn(&self, pane: Pane) -> bool {
        match pane {
            Pane::View => !self.single && self.collection.is_none(),
            Pane::Collection => self.viewing().is_some(),
            Pane::Todos => self.single,
            _ => self.sections().contains(&pane),
        }
    }

    /// The todo a heading should hand over to: the first one *after* the
    /// selected heading's line, and when that section has none, the nearest one
    /// before it. `None` only when the file has no todos at all.
    pub fn task_after_outline(&self) -> Option<usize> {
        let heading = self.headings().get(self.outline_idx)?.line;
        let tasks = self.tasks();
        if tasks.is_empty() {
            return None;
        }
        let after = tasks.iter().position(|task| task.line > heading);
        let before = tasks.iter().rposition(|task| task.line < heading);
        Some(after.or(before).unwrap_or(0))
    }

    /// Cross into the right-hand panel, remembering the section to come back to.
    pub fn enter_view(&mut self) -> Result<(), Error> {
        if self.crossing() {
            return Ok(());
        }
        // a collection row steps into the collection that is already drawn beside it
        if self
            .selected_row()
            .is_some_and(|row| row.collection().is_some())
        {
            self.last_sidebar = Pane::Files;
            // it is loaded by the preview; `→` only hands it the keyboard
            self.preview_row()?;
            self.pane = Pane::Collection;
            return Ok(());
        }
        // the outline hands its heading's todo to whichever panel is on the right, so
        // `→` from the index lands on the todo under that heading either way
        if self.pane == Pane::Outline {
            self.jump_from_outline();
        }
        self.last_sidebar = self.pane;
        // a single-file session's right-hand panel is the todos of the one file; a
        // folder session's is the file view (RFC-001)
        self.pane = if self.single { Pane::Todos } else { Pane::View };
        Ok(())
    }

    /// Whether the cursor is in the right-hand panel: the file view, a collection, or
    /// the todos panel of a single-file session.
    pub fn crossing(&self) -> bool {
        matches!(self.pane, Pane::View | Pane::Collection)
            || (self.single && self.pane == Pane::Todos)
    }

    /// Cross back out of the right-hand panel.
    pub fn leave_view(&mut self) {
        // the collection stays on screen: it is what the highlighted row is about,
        // and the row is still highlighted
        if self.crossing() {
            self.pane = self.last_sidebar;
        }
    }

    /// Mark the heading the selected todo sits under, so the outline follows the
    /// file view the same way the view follows the outline.
    pub fn sync_outline(&mut self) {
        let Some(task) = self.current_task() else {
            return;
        };
        let headings = self.headings();
        if headings.is_empty() {
            return;
        }
        self.outline_idx = headings
            .iter()
            .rposition(|heading| heading.line < task.line)
            .unwrap_or(0);
    }

    /// Hand the outline's heading to the todos: the first one in its section.
    pub fn jump_from_outline(&mut self) {
        if let Some(index) = self.task_after_outline() {
            self.task_idx = index;
        }
    }

    /// The prompt being typed, when a todo is being edited in place: the row itself is
    /// the field, which is what `e` does. A new todo is typed in a field over the
    /// screen, into the empty row that is waiting for it — see `add`.
    pub fn editing(&self) -> Option<&crate::state::prompt::Prompt> {
        match &self.mode {
            crate::state::prompt::Mode::Editing(prompt) => Some(prompt),
            _ => None,
        }
    }

    /// Where `tab` (forward) or `shift+tab` (backward) would land, so the hint
    /// can name the panel instead of saying "section".
    ///
    /// `None` means "nowhere": the file view is not part of the sidebar, so from
    /// there `tab` does nothing and `←` is the way out.
    pub fn focus_target(&self, forward: bool) -> Option<Pane> {
        if self.pane == Pane::View {
            return None;
        }
        let sections = self.sections();
        if sections.is_empty() {
            return None;
        }
        let position = sections.iter().position(|pane| *pane == self.pane)?;
        let next = if forward {
            (position + 1) % sections.len()
        } else {
            (position + sections.len() - 1) % sections.len()
        };
        sections.get(next).copied()
    }

    pub fn tasks(&self) -> Vec<Task> {
        self.doc.tasks()
    }

    pub fn current_task(&self) -> Option<Task> {
        if self.collection.is_some() {
            // the item knows its line; the open document is its origin file
            let line = self.items.get(self.task_idx)?.line;
            return self.tasks().into_iter().find(|task| task.line == line);
        }
        self.tasks().get(self.task_idx).cloned()
    }

    /// The write this one undoes: the nearest older write to the same line that is
    /// exactly its inverse. A write that merely came before it is not a dependency.
    pub fn dependency(&self, global: usize) -> Option<usize> {
        let entry = self.history.get(global)?;
        let mine = entry.change();
        (global + 1..self.history.len()).find(|index| {
            let older = &self.history[*index];
            if older.path != entry.path || older.line() != entry.line() {
                return false;
            }
            match (mine.as_ref(), older.change()) {
                (Some(mine), Some(older)) => mine.undoes(&older),
                // an insert and a delete carry their own text, which is the proof
                _ => older.forward == entry.forward.clone().flip(),
            }
        })
    }

    /// Whether `global` hangs on `target`, directly or through the writes between.
    pub fn depends_on(&self, global: usize, target: usize) -> bool {
        let mut at = self.dependency(global);
        while let Some(index) = at {
            if index == target {
                return true;
            }
            at = self.dependency(index);
        }
        false
    }

    /// The writes that hang on this one, newest first: undoing it undoes them too.
    pub fn dependents(&self, global: usize) -> Vec<usize> {
        (0..global)
            .filter(|index| self.depends_on(*index, global))
            .collect()
    }

    /// Global indices into `history` for the open file, newest first.
    pub fn file_history(&self) -> Vec<usize> {
        let path = self.current().path.clone();
        self.history
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.path == path)
            .map(|(index, _)| index)
            .collect()
    }

    /// How many writes this session recorded against a file: what the panel's `h3`
    /// is counting.
    pub fn history_count(&self, path: &Path) -> usize {
        self.history
            .iter()
            .filter(|entry| entry.path == path)
            .count()
    }

    /// How a path is shown: relative to where the app was opened, so a scan of
    /// a folder reads as `a/TODOs.md` and a single file reads as its name.
    pub fn display(&self, path: &Path) -> String {
        path.strip_prefix(&self.root).map_or_else(
            |_| {
                path.file_name().map_or_else(
                    || path.display().to_string(),
                    |name| name.to_string_lossy().into_owned(),
                )
            },
            |relative| {
                if relative.as_os_str().is_empty() {
                    self.display(path.parent().unwrap_or(path))
                } else {
                    relative.display().to_string()
                }
            },
        )
    }

    /// The folders a file sits in, relative to the root: the levels the panel
    /// draws as a tree. A file at the root has none.
    pub fn folders(&self, path: &Path) -> Vec<String> {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .parent()
            .map(|folder| {
                folder
                    .components()
                    .filter_map(|part| match part {
                        std::path::Component::Normal(name) => {
                            Some(name.to_string_lossy().into_owned())
                        }
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The folder the new-file field starts in: the selected file's own — nothing
    /// at the root, so the first thing typed is the new folder's name, and `web/`
    /// below it, so typing continues *inside* the folder that is already open. The
    /// `./` and the file name are the field's own frame, not part of what is typed.
    pub fn selected_folder(&self) -> String {
        let folders = self.folders(&self.current().path);
        if folders.is_empty() {
            String::new()
        } else {
            format!("{}/", folders.join("/"))
        }
    }

    /// Make a folder and its todo file, then open it.
    ///
    /// The existence check is repeated here because a field is not the authority on
    /// what is on disk, and this is the last moment before a write.
    pub fn create_file(&mut self, folder: &Path) -> Result<(), Error> {
        let file = folder.join(FILE_NAME);
        if let Err(error) = disk::create(&file, &text::new_file(&Self::title_for(folder))) {
            // the message names the path the way the panel shows it, not the
            // absolute one the disk would report
            return Err(match error {
                Error::Io(io) if io.kind() == std::io::ErrorKind::AlreadyExists => {
                    Error::Ops(text::already_there(&self.display(&file)))
                }
                other => other,
            });
        }
        self.rescan(&file)
    }

    /// The heading a new file starts with: the folder it lives in, which is what
    /// everybody wrote by hand anyway.
    fn title_for(folder: &Path) -> String {
        folder
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
    }

    /// Delete a todo file and open a neighbour.
    ///
    /// The list is rebuilt from the disk afterwards, because the list *is* what is
    /// on disk. A write recorded against the deleted path can never be undone —
    /// the lines it anchored to are gone — so those entries go with the file.
    pub fn delete_file(&mut self, path: &Path) -> Result<(), Error> {
        disk::delete(path)?;
        self.history.retain(|entry| entry.path != path);
        let opened = self
            .files
            .get(self.file_idx)
            .map(|entry| entry.path.clone());
        let scan = discover::scan(&self.root)?;
        self.files = scan.entries;
        self.warnings = scan.warnings;
        if self.files.is_empty() {
            // there is nothing left to be about
            self.quit = true;
            return Ok(());
        }
        self.file_idx = self.file_idx.min(self.files.len() - 1);
        self.rebuild_rows();
        if opened.as_deref() == Some(path) {
            self.open_current()?;
        }
        Ok(())
    }

    /// Show what the highlighted row is about.
    ///
    /// A file row opens its document; a collection row gathers its todos and opens
    /// the file the first of them lives in. Either way the right-hand panel follows
    /// the cursor in the file list, so a collection is read before it is stepped
    /// into — and `→` only decides who owns the keyboard.
    pub fn preview_row(&mut self) -> Result<(), Error> {
        match self.selected_row().cloned() {
            Some(Row::File(index)) => {
                if self.collection.is_some() {
                    self.collection = None;
                    self.items.clear();
                }
                if index == self.file_idx {
                    return Ok(());
                }
                self.file_idx = index;
                self.open_current()
            }
            Some(Row::Collection(collection)) => {
                // the same collection, already gathered: keep the place in it
                if self.collection.as_ref() == Some(&collection) {
                    return Ok(());
                }
                self.load_collection(&collection)
            }
            None => Ok(()),
        }
    }

    /// Gather a collection and open the file its first todo lives in.
    pub(crate) fn load_collection(&mut self, collection: &Collection) -> Result<(), Error> {
        let items = collections::gather(&self.files, collection)?;
        self.collection = Some(collection.clone());
        self.items = items;
        self.task_idx = 0;
        // the document under the cursor is the origin file of what is highlighted,
        // so an edit and the history panel already point at the right file
        if let Some(first) = self.items.first() {
            if let Some(index) = self.files.iter().position(|file| file.path == first.path) {
                self.file_idx = index;
                self.open_current()?;
            }
        }
        Ok(())
    }

    /// The file a collection's rows live in, for the panel titles.
    pub fn items(&self) -> &[Item] {
        &self.items
    }

    /// How much of a collection is done.
    pub fn collection_stats(&self) -> Stats {
        Stats {
            all: self.items.len(),
            done: self.items.iter().filter(|item| item.done).count(),
            tags: 0,
            assignees: 0,
        }
    }

    /// Put the panel's cursor on the row for this file, after something else moved
    /// the open file.
    /// Take the keyboard to a todo: open the file it lives in, put the cursor on
    /// that todo, put the index on the heading above it, and show the file's view.
    ///
    /// Two things ask for this — a search hit and a todo read in a collection —
    /// because it is one idea. A collection is a way of *reading* files, so landing
    /// in one of them ends the reading: it stops being viewed, the file's own
    /// headings come back, and the file panel's cursor moves to that file.
    pub fn jump_to(&mut self, path: &Path, line: usize) -> Result<(), Error> {
        self.focus_file(path);
        self.collection = None;
        self.items.clear();
        self.open_current()?;
        self.task_idx = self
            .tasks()
            .iter()
            .position(|task| task.line == line)
            .unwrap_or(0);
        self.sync_outline();
        self.pane = if self.single { Pane::Todos } else { Pane::View };
        Ok(())
    }

    pub fn focus_file(&mut self, path: &Path) {
        if let Some(index) = self.files.iter().position(|file| file.path == path) {
            self.file_idx = index;
            if let Some(row) = self.rows.iter().position(|row| row.file() == Some(index)) {
                self.row_idx = row;
            }
        }
    }

    /// Show a document that was just read.
    ///
    /// One place, because three things have to agree: what is on screen, what a
    /// save is allowed to start from, and where the cursor is — a cursor left
    /// pointing into a file that is no longer open is how a key press ends up
    /// touching the wrong line.
    fn show(&mut self, loaded: Loaded) {
        self.doc = Doc::parse_at(&loaded.path, &loaded.text);
        self.baseline = loaded;
        self.task_idx = 0;
        self.outline_idx = 0;
        self.history_idx = 0;
    }

    /// Read the file the cursor is on, as switching to it would.
    pub(crate) fn open_current(&mut self) -> Result<(), Error> {
        let loaded = store::load(&self.current().path)?;
        self.show(loaded);
        Ok(())
    }

    /// Scan the root again, as an outside change would, and open `focus`.
    pub(crate) fn rescan(&mut self, focus: &Path) -> Result<(), Error> {
        let scan = discover::scan_scope(&self.root, self.scope())?;
        self.files = scan.entries;
        self.warnings = scan.warnings;
        // the scan lists files that hold a todo, and a file just made holds none.
        // It is listed here as `0/0` so the panel shows what was just created.
        // shortcut: an empty file falls out of the list on the next rescan; remember
        // the paths made this session if that ever bites.
        if !self.files.iter().any(|entry| entry.path == focus) {
            self.files.push(FileEntry {
                path: focus.to_path_buf(),
                source: Source::of(focus),
                stats: Stats::default(),
                too_long: false,
                collections: Vec::new(),
            });
            self.files.sort_by(|left, right| {
                aricma_todo_lib::discover::order::compare(&left.path, &right.path)
            });
        }
        let Some(index) = self.files.iter().position(|entry| entry.path == focus) else {
            return Err(Error::Ops(format!(
                "{}: not found after creating it",
                focus.display()
            )));
        };
        self.file_idx = index;
        self.rebuild_rows();
        self.open_current()
    }

    /// Whether the open file is a comment one: todo comments in code, whose
    /// todos can be edited, copied, priced and deleted, and nothing else. The keys
    /// that need a document are refused rather than hidden in silence (RFC-015).
    pub fn comment(&self) -> bool {
        self.doc.source() == Source::Comment
    }

    /// Which files the scan looks at: the ones named for todos, or every markdown
    /// file in the tree when `h` has been pressed.
    pub const fn scope(&self) -> discover::Scope {
        if self.show_all {
            discover::Scope::EveryMarkdown
        } else {
            discover::Scope::Named
        }
    }

    /// Whether a modal has the keyboard. One at a time: a modal owns the keys until
    /// it is left, and nothing opens on top of it.
    pub fn modal(&self) -> bool {
        self.mode != Mode::Normal
    }

    /// Whether something is drawn **over** the screen. A field typed into a row (`e`)
    /// owns the keyboard without floating, so the screen behind it stays as bright as
    /// it is; a new todo's field floats over the row that is waiting for it.
    pub const fn floating(&self) -> bool {
        !matches!(&self.mode, Mode::Normal | Mode::Editing(_))
    }

    /// An answer to "why can't I do that?" — a rule, not a failure.
    pub fn set_warning(&mut self, text: impl Into<String>) {
        self.message = Some(Message::warning(text));
    }

    /// Something worth telling the reader, where nothing is wrong.
    pub fn set_info(&mut self, text: impl Into<String>) {
        self.message = Some(Message::info(text));
    }

    /// Something that failed: a write that did not happen.
    pub fn set_error(&mut self, text: impl Into<String>) {
        self.message = Some(Message::failed(text));
    }

    /// Put a message on the history row it is about.
    pub fn warn_on(&mut self, row: usize, text: impl Into<String>) {
        if let Some(entry) = self.history.get_mut(row) {
            entry.message = Some(Message::warning(text));
        }
    }

    /// The panel message's own words, for a test that asserts what was said.
    #[cfg(test)]
    pub fn message_text(&self) -> String {
        self.message
            .as_ref()
            .map(|message| message.text.clone())
            .unwrap_or_default()
    }

    pub fn clear_message(&mut self) {
        self.message = None;
    }

    /// What this history row has to say for itself: what just happened to it, else
    /// why it cannot be toggled at all.
    pub fn message_for(&self, global: usize) -> Option<Message> {
        if let Some(message) = self
            .history
            .get(global)
            .and_then(|entry| entry.message.clone())
        {
            return Some(message);
        }
        // a row whose dependency is undone cannot go back on the line. That is a
        // fact about the row the cursor is on, so it is derived, not stored, and
        // only the selected row says it — like a field that is only validated
        // while it has the focus
        if self.pane != Pane::History {
            return None;
        }
        if *self.file_history().get(self.history_idx)? != global {
            return None;
        }
        let under = self.dependency(global)?;
        let entry = self.history.get(global)?;
        (!entry.applied && !self.history[under].applied).then(|| Message::warning(text::BLOCKED))
    }

    /// The message that belongs to the panel rather than to one of its rows.
    pub fn panel_message(&self) -> Option<&Message> {
        // a success is drawn in the header, where the window says what is going
        // on; a panel's own messages are the ones that explain something about it
        self.message
            .as_ref()
            .filter(|message| !message.is_success())
    }

    /// What worked, for the header's right-hand end.
    pub fn success_message(&self) -> Option<&Message> {
        self.message.as_ref().filter(|message| message.is_success())
    }

    /// A save hit a file that changed under it: ask, because only the person knows
    /// which version is the one they meant.
    pub fn conflict(&mut self) {
        self.mode = Mode::Conflicted;
    }

    /// Take the file's version: reload it and drop our own and the history that was
    /// anchored to it.
    pub fn take_theirs(&mut self) -> Result<(), Error> {
        self.reload()?;
        self.set_info(text::took_theirs());
        Ok(())
    }

    /// The baseline a successful save establishes.
    pub fn saved(&mut self, loaded: Loaded) {
        self.baseline = loaded;
    }

    /// Re-read the open file when something else wrote it, so the screen is
    /// never showing a stale document. A write of our own updates the baseline,
    /// so it never looks like a change.
    ///
    /// Refused while a prompt is open: replacing the document under someone who
    /// is typing would lose what they typed, and the save would then be refused
    /// with the usual "changed on disk" message.
    pub fn refresh_from_disk(&mut self) -> Result<(), Error> {
        if self.mode != Mode::Normal {
            return Ok(());
        }
        if store::load(&self.current().path)?.text == self.baseline.text {
            return Ok(());
        }
        self.reload()
    }

    /// Re-read the open file from disk and forget the history, which was
    /// anchored to the text that is now gone.
    pub fn reload(&mut self) -> Result<(), Error> {
        self.open_current()?;
        // a reload drops the history: the lines every entry anchors to are gone
        self.history.clear();
        self.history_idx = 0;
        self.task_idx = 0;
        self.mode = Mode::Normal;
        Ok(())
    }

    /// Move the panel's cursor, opening the file it lands on.
    ///
    /// A collection row opens nothing by itself: `→` is what reads it. Landing on a
    /// file is what opens it, because the document under the cursor is the one the
    /// keys would write to.
    pub fn select_file(&mut self, delta: isize) -> Result<(), Error> {
        let Some(next) = self.reachable(delta) else {
            // bailing out on a blocked file would leave the cursor stuck on it, so
            // the only thing left to do is say where the limit is
            let at = shift(self.row_idx, delta, self.rows.len());
            if let Some(Row::File(index)) = self.rows.get(at) {
                let shown = self.display(&self.files[*index].path);
                self.set_warning(text::too_long(&shown, aricma_todo_lib::limits::MAX_LINES));
            }
            return Ok(());
        };
        self.row_idx = next;
        self.preview_row()
    }

    pub fn save_action(&self) -> Action {
        Action::Save {
            text: self.doc.render(),
        }
    }

    /// Keep the collection view true to a write just made.
    ///
    /// The gathered items are a snapshot, and a collection has no document of its
    /// own to re-read. The write says what it did: a line that was deleted leaves
    /// the collection, and anything else is read back from the document, because
    /// after a delete the lines have moved and a line is no longer an identity.
    fn sync_item(&mut self, forward: &Inverse) {
        if self.collection.is_none() {
            return;
        }
        if matches!(forward, Inverse::Remove { .. }) {
            if self.task_idx < self.items.len() {
                self.items.remove(self.task_idx);
            }
            self.task_idx = self.task_idx.min(self.items.len().saturating_sub(1));
            return;
        }
        let Some(task) = self.current_task() else {
            return;
        };
        if let Some(item) = self.items.get_mut(self.task_idx) {
            item.body.clone_from(&task.body);
            // the box and the bullet are part of what a collection draws, so a
            // check has to reach them too — they are the line as it now is
            item.lead.clone_from(&task.lead());
            item.done = task.done;
        }
    }

    /// Record a write and ask for it to be saved.
    pub fn record(&mut self, label: String, forward: Inverse) -> Action {
        self.sync_item(&forward);
        self.history.insert(
            0,
            Entry {
                path: self.current().path.clone(),
                label,
                forward,
                applied: true,
                message: None,
            },
        );
        self.history_idx = 0;
        self.refresh_stats();
        self.save_action()
    }

    /// Keep the sidebar's numbers true after an edit.
    pub(crate) fn refresh_stats(&mut self) {
        let stats = self.doc.stats();
        if let Some(file) = self.files.get_mut(self.file_idx) {
            file.stats = stats;
        }
    }

    /// `task_idx` clamped to the tasks that still exist.
    pub fn clamp_task(&mut self) {
        let count = if self.collection.is_some() {
            self.items.len()
        } else {
            self.tasks().len()
        };
        self.task_idx = self.task_idx.min(count.saturating_sub(1));
    }
}
