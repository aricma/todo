//! The TUI's state: three panes, one open file, and the session history.
//!
//! Nothing here touches the filesystem or the terminal. A key press either
//! changes in-memory state or returns an `Action` for the caller to perform —
//! which is what makes every rule testable without a pty.

use std::path::{Path, PathBuf};

use aricma_todo_lib::discover::{self, Entry as FileEntry};
use aricma_todo_lib::doc::Doc;
use aricma_todo_lib::error::Error;
use aricma_todo_lib::model::{Heading, Inverse, Task};
use aricma_todo_lib::store::{self, Loaded};

use super::action::{Action, Pane};
use super::entry::Entry;
use super::prompt::Mode;

/// Everything the screen needs, and nothing it does not.
pub struct App {
    pub files: Vec<FileEntry>,
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
    /// The sidebar section to come back to when leaving the file view.
    pub last_sidebar: Pane,
    pub history: Vec<Entry>,
    pub mode: Mode,
    pub message: Option<String>,
    pub quit: bool,
}

impl App {
    /// Scan a path and open its first file.
    pub fn open(path: &Path) -> Result<Self, Error> {
        let files = discover::scan(path)?;
        let Some(first) = files.first() else {
            return Err(Error::Ops(format!(
                "{}: no todo files found",
                path.display()
            )));
        };
        let baseline = store::load(&first.path)?;
        let single = path.is_file();
        Ok(Self {
            doc: Doc::parse(&baseline.text),
            files,
            file_idx: 0,
            task_idx: 0,
            outline_idx: 0,
            history_idx: 0,
            root: path.to_path_buf(),
            single,
            last_sidebar: if single { Pane::Todos } else { Pane::Files },
            pane: if single { Pane::Todos } else { Pane::Files },
            baseline,
            history: Vec::new(),
            mode: Mode::Normal,
            message: None,
            quit: false,
        })
    }

    pub fn current(&self) -> &FileEntry {
        &self.files[self.file_idx]
    }

    /// The document's headings, in file order.
    pub fn headings(&self) -> Vec<Heading> {
        self.doc.headings()
    }

    /// An outline of one heading says nothing, so the section is hidden then.
    pub fn has_outline(&self) -> bool {
        self.headings().len() > 1
    }

    /// The sidebar sections, in the order `Tab` walks them. The file view is
    /// deliberately not one: `→` and `←` are what cross between the two.
    pub fn sections(&self) -> Vec<Pane> {
        let mut sections = Vec::new();
        if !self.single {
            sections.push(Pane::Files);
        }
        if self.has_outline() {
            sections.push(Pane::Outline);
        }
        if self.single {
            sections.push(Pane::Todos);
        }
        sections.push(Pane::History);
        sections
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

    /// Cross into the file view, remembering the section to come back to.
    pub fn enter_view(&mut self) {
        if self.single || self.pane == Pane::View {
            return;
        }
        if self.pane == Pane::Outline {
            self.jump_from_outline();
        }
        self.last_sidebar = self.pane;
        self.pane = Pane::View;
    }

    /// Cross back out of the file view.
    pub fn leave_view(&mut self) {
        if self.pane == Pane::View {
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

    /// The prompt being typed, when a todo is being edited in place.
    pub fn editing(&self) -> Option<&crate::prompt::Prompt> {
        match &self.mode {
            crate::prompt::Mode::Editing(prompt) => Some(prompt),
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
        self.tasks().get(self.task_idx).cloned()
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

    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = Some(message.into());
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
        self.reload()?;
        self.set_message("reloaded from disk");
        Ok(())
    }

    /// Re-read the open file from disk and forget the history, which was
    /// anchored to the text that is now gone.
    pub fn reload(&mut self) -> Result<(), Error> {
        let loaded = store::load(&self.current().path)?;
        self.doc = Doc::parse(&loaded.text);
        self.baseline = loaded;
        self.history.clear();
        self.history_idx = 0;
        self.task_idx = 0;
        self.mode = Mode::Normal;
        Ok(())
    }

    /// Open another file, keeping the pane and dropping the selections.
    pub fn select_file(&mut self, delta: isize) -> Result<(), Error> {
        let next = shift(self.file_idx, delta, self.files.len());
        if next == self.file_idx {
            return Ok(());
        }
        self.file_idx = next;
        let loaded = store::load(&self.current().path)?;
        self.doc = Doc::parse(&loaded.text);
        self.baseline = loaded;
        self.task_idx = 0;
        self.history_idx = 0;
        Ok(())
    }

    pub fn save_action(&self) -> Action {
        Action::Save {
            path: self.current().path.clone(),
            text: self.doc.render(),
        }
    }

    /// Record a write and ask for it to be saved.
    pub fn record(&mut self, label: String, forward: Inverse) -> Action {
        self.history.insert(
            0,
            Entry {
                path: self.current().path.clone(),
                label,
                forward,
                applied: true,
            },
        );
        self.history_idx = 0;
        self.refresh_stats();
        self.save_action()
    }

    /// Keep the sidebar's numbers true after an edit.
    fn refresh_stats(&mut self) {
        let stats = self.doc.stats();
        if let Some(file) = self.files.get_mut(self.file_idx) {
            file.stats = stats;
        }
    }

    /// `task_idx` clamped to the tasks that still exist.
    pub fn clamp_task(&mut self) {
        self.task_idx = self.task_idx.min(self.tasks().len().saturating_sub(1));
    }
}

/// Move by `delta` inside `0..len`, clamped.
pub fn shift(current: usize, delta: isize, len: usize) -> usize {
    let last = len.saturating_sub(1);
    if delta < 0 {
        current.saturating_sub(delta.unsigned_abs())
    } else {
        (current + delta.unsigned_abs()).min(last)
    }
}

#[cfg(test)]
mod tests {
    use aricma_todo_lib::doc::Doc;
    use ratatui::crossterm::event::KeyCode;

    use crate::action::{Action, Pane};
    use crate::prompt::Mode;
    use crate::testing::{app, press, save_text, single, type_text};

    #[test]
    fn j_and_k_move_and_clamp_inside_the_focused_pane() {
        let (_tmp, mut app) = app(&[(
            "TODOs.md",
            "- [ ] a
- [ ] b
- [ ] c
",
        )]);
        app.pane = Pane::Todos;
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Down);
        assert_eq!(app.task_idx, 2);
        press(&mut app, KeyCode::Down);
        assert_eq!(app.task_idx, 2, "clamped at the last task");
        press(&mut app, KeyCode::Char('G'));
        press(&mut app, KeyCode::Up);
        assert_eq!(app.task_idx, 1);
    }

    #[test]
    fn the_file_view_is_never_a_section() {
        let (_tmp, mut app) = app(&[("TODOs.md", "# one\n\n- [ ] a\n\n## two\n\n- [ ] b\n")]);
        for _ in 0..6 {
            press(&mut app, KeyCode::Tab);
            assert_ne!(
                app.pane,
                Pane::Todos,
                "a folder session has no todos section"
            );
        }
    }

    #[test]
    fn the_outline_is_a_section_only_when_there_are_two_headings() {
        let (_tmp, mut app) = app(&[("TODOs.md", "# only one\n- [ ] a\n")]);
        assert!(!app.has_outline());
        assert_eq!(app.sections(), [Pane::Files, Pane::History]);

        app.doc = Doc::parse("# one\n\n## two\n- [ ] a\n");
        assert!(app.has_outline());
        assert_eq!(app.sections(), [Pane::Files, Pane::Outline, Pane::History]);
        press(&mut app, KeyCode::Tab);
        assert_eq!(app.pane, Pane::Outline);
    }

    #[test]
    fn a_single_file_session_starts_on_its_todos_and_has_no_file_list() {
        let (_tmp, mut app) = single("- [ ] a\n");
        assert_eq!(app.pane, Pane::Todos);
        assert_eq!(app.sections(), [Pane::Todos, Pane::History]);
        press(&mut app, KeyCode::Tab);
        assert_eq!(app.pane, Pane::History);
        press(&mut app, KeyCode::BackTab);
        assert_eq!(app.pane, Pane::Todos);
    }

    #[test]
    fn tab_loops_the_sidebar_and_the_arrows_cross_to_the_view() {
        let (_tmp, mut app) = app(&[("TODOs.md", "# one\n\n- [ ] a\n\n## two\n\n- [ ] b\n")]);
        assert_eq!(app.sections(), [Pane::Files, Pane::Outline, Pane::History]);
        assert_eq!(app.pane, Pane::Files);

        press(&mut app, KeyCode::Tab);
        assert_eq!(app.pane, Pane::Outline);
        press(&mut app, KeyCode::Tab);
        assert_eq!(app.pane, Pane::History);
        press(&mut app, KeyCode::Tab);
        assert_eq!(app.pane, Pane::Files, "tab loops the sidebar");

        // the arrows cross the gap, and come back to where they left
        press(&mut app, KeyCode::Right);
        assert_eq!(app.pane, Pane::View);
        press(&mut app, KeyCode::Left);
        assert_eq!(app.pane, Pane::Files);

        press(&mut app, KeyCode::Tab); // outline
        press(&mut app, KeyCode::Right);
        assert_eq!(app.pane, Pane::View);
        press(&mut app, KeyCode::Left);
        assert_eq!(app.pane, Pane::Outline, "back to the section it left");

        // tab is sidebar navigation, so inside the view it does nothing at all:
        // the left arrow is the way out, and it goes where you came from
        press(&mut app, KeyCode::Right);
        assert_eq!(app.pane, Pane::View);
        press(&mut app, KeyCode::Tab);
        assert_eq!(app.pane, Pane::View, "tab does not move out of the view");
        press(&mut app, KeyCode::BackTab);
        assert_eq!(app.pane, Pane::View, "nor backwards");
        assert_eq!(app.focus_target(true), None);
        assert_eq!(app.focus_target(false), None);
        press(&mut app, KeyCode::Left);
        assert_eq!(app.pane, Pane::Outline, "left returns to the outline");
    }

    #[test]
    fn moving_the_todo_moves_the_outline_to_the_heading_above_it() {
        let (_tmp, mut app) = app(&[(
            "TODOs.md",
            "# inbox\n\n- [ ] a\n- [ ] b\n\n## later\n\n- [ ] c\n- [ ] d\n",
        )]);
        app.pane = Pane::View;
        assert_eq!(app.outline_idx, 0, "a is under the first heading");

        press(&mut app, KeyCode::Down); // b
        assert_eq!(app.current_task().unwrap().title, "b");
        assert_eq!(app.outline_idx, 0, "still the first section");

        press(&mut app, KeyCode::Down); // c
        assert_eq!(app.current_task().unwrap().title, "c");
        assert_eq!(app.outline_idx, 1, "the outline marks ## later now");

        press(&mut app, KeyCode::Up); // b
        assert_eq!(app.outline_idx, 0, "and back again");

        press(&mut app, KeyCode::Char('G')); // d
        assert_eq!(app.outline_idx, 1);
    }

    #[test]
    fn a_todo_above_every_heading_marks_the_first_one() {
        let (_tmp, mut app) = app(&[(
            "TODOs.md",
            "- [ ] loose\n\n## a\n\n- [ ] b\n\n## c\n\n- [ ] d\n",
        )]);
        app.pane = Pane::View;
        assert_eq!(app.outline_idx, 0, "no heading above it, so the first one");
        press(&mut app, KeyCode::Down); // b, under ## a
        assert_eq!(app.outline_idx, 0);
        press(&mut app, KeyCode::Down); // d, under ## c
        assert_eq!(app.outline_idx, 1);
    }

    #[test]
    fn moving_the_outline_hands_over_to_that_sections_first_todo() {
        let (_tmp, mut app) = app(&[(
            "TODOs.md",
            "# one\n\n- [ ] a\n\n## two\n\n- [ ] b\n- [ ] c\n",
        )]);
        app.pane = Pane::Outline;
        assert_eq!(app.current_task().unwrap().title, "a");
        press(&mut app, KeyCode::Down); // "## two"
        assert_eq!(app.current_task().unwrap().title, "b");

        // the view is where the todos are walked, and the outline handed it "b"
        press(&mut app, KeyCode::Right);
        assert_eq!(app.pane, Pane::View);
        assert_eq!(app.current_task().unwrap().title, "b");
        press(&mut app, KeyCode::Down);
        assert_eq!(app.current_task().unwrap().title, "c");
    }

    #[test]
    fn a_heading_with_no_todos_hands_over_to_the_nearest_one_before() {
        let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n\n## empty\n\n## end\n")]);
        app.pane = Pane::Outline;
        press(&mut app, KeyCode::Char('G')); // "## end", nothing after it
        assert_eq!(app.outline_idx, 1);
        assert_eq!(app.current_task().unwrap().title, "a");
    }

    #[test]
    fn the_outline_moves_between_headings() {
        let (_tmp, mut app) = app(&[("TODOs.md", "# one\n\n## two\n\n### three\n\n- [ ] a\n")]);
        app.pane = Pane::Outline;
        assert_eq!(app.outline_idx, 0);
        press(&mut app, KeyCode::Down);
        assert_eq!(app.outline_idx, 1);
        press(&mut app, KeyCode::Char('G'));
        assert_eq!(app.outline_idx, 2);
        press(&mut app, KeyCode::Down);
        assert_eq!(app.outline_idx, 2, "clamped at the last heading");
        press(&mut app, KeyCode::Char('g'));
        assert_eq!(app.outline_idx, 0);
    }

    #[test]
    fn x_and_space_check_and_uncheck_a_task() {
        let (_tmp, mut app) = app(&[(
            "TODOs.md",
            "- [ ] (A) two  spaces #x @y  
",
        )]);
        app.pane = Pane::Todos;
        let text = save_text(press(&mut app, KeyCode::Char('x')));
        assert_eq!(
            text,
            "- [x] (A) two  spaces #x @y  
"
        );
        let text = save_text(press(&mut app, KeyCode::Char(' ')));
        assert_eq!(
            text,
            "- [ ] (A) two  spaces #x @y  
"
        );
    }

    #[test]
    fn the_letter_a_todo_already_carries_clears_the_priority() {
        let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
        app.pane = Pane::View;

        assert_eq!(
            save_text(press(&mut app, KeyCode::Char('B'))),
            "- [ ] (B) a\n"
        );
        assert_eq!(app.history[0].label, "priority B · a");

        // the same letter again removes it
        assert_eq!(save_text(press(&mut app, KeyCode::Char('B'))), "- [ ] a\n");
        assert_eq!(app.history[0].label, "priority clear · a");

        // and a different letter replaces it
        press(&mut app, KeyCode::Char('B'));
        assert_eq!(
            save_text(press(&mut app, KeyCode::Char('C'))),
            "- [ ] (C) a\n"
        );
        assert_eq!(app.history[0].label, "priority C · a");

        // pressing C again clears, C again sets it back
        assert_eq!(save_text(press(&mut app, KeyCode::Char('C'))), "- [ ] a\n");
        assert_eq!(
            save_text(press(&mut app, KeyCode::Char('C'))),
            "- [ ] (C) a\n"
        );
    }

    #[test]
    fn uppercase_a_to_d_set_priority_and_zero_clears_it() {
        let (_tmp, mut app) = app(&[(
            "TODOs.md",
            "- [ ] a #t
",
        )]);
        app.pane = Pane::Todos;
        assert_eq!(
            save_text(press(&mut app, KeyCode::Char('B'))),
            "- [ ] (B) a #t
"
        );
        assert_eq!(
            save_text(press(&mut app, KeyCode::Char('0'))),
            "- [ ] a #t
"
        );
        assert_eq!(press(&mut app, KeyCode::Char('0')), None);
    }

    #[test]
    fn d_asks_then_deletes_on_y_and_cancels_on_n() {
        let (_tmp, mut app) = app(&[(
            "TODOs.md",
            "- [ ] a
- [ ] b
",
        )]);
        app.pane = Pane::Todos;
        assert_eq!(press(&mut app, KeyCode::Char('d')), None);
        assert_eq!(app.mode, Mode::Confirming);
        assert_eq!(press(&mut app, KeyCode::Char('n')), None);
        assert_eq!(app.mode, Mode::Normal);

        press(&mut app, KeyCode::Char('d'));
        let text = save_text(press(&mut app, KeyCode::Char('y')));
        assert_eq!(
            text,
            "- [ ] b
"
        );
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn escape_cancels_a_delete() {
        let (_tmp, mut app) = app(&[(
            "TODOs.md", "- [ ] a
",
        )]);
        app.pane = Pane::Todos;
        press(&mut app, KeyCode::Char('d'));
        assert_eq!(press(&mut app, KeyCode::Esc), None);
        assert_eq!(app.mode, Mode::Normal);
        assert_eq!(
            app.doc.render(),
            "- [ ] a
"
        );
    }

    #[test]
    fn a_opens_an_empty_prompt_and_enter_appends_what_was_typed() {
        let (_tmp, mut app) = app(&[(
            "TODOs.md", "- [ ] a
",
        )]);
        app.pane = Pane::Todos;
        press(&mut app, KeyCode::Char('a'));
        assert_eq!(
            app.mode,
            Mode::Adding(super::super::prompt::Prompt::new(""))
        );
        type_text(&mut app, "ship it #x @y");
        let text = save_text(press(&mut app, KeyCode::Enter));
        assert_eq!(
            text,
            "- [ ] a
- [ ] ship it #x @y
"
        );
        assert_eq!(app.task_idx, 1);
    }

    #[test]
    fn a_refuses_an_empty_body_and_writes_nothing() {
        let (_tmp, mut app) = app(&[(
            "TODOs.md", "- [ ] a
",
        )]);
        app.pane = Pane::Todos;
        press(&mut app, KeyCode::Char('a'));
        assert_eq!(press(&mut app, KeyCode::Enter), None);
        assert_eq!(
            app.doc.render(),
            "- [ ] a
"
        );
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn e_prefills_the_body_and_writes_it_back_as_typed() {
        let (_tmp, mut app) = app(&[(
            "TODOs.md",
            "  * [x] (A) old #tag @who
",
        )]);
        app.pane = Pane::Todos;
        press(&mut app, KeyCode::Char('e'));
        assert_eq!(
            app.mode,
            Mode::Editing(crate::prompt::Prompt::new("(A) old #tag @who"))
        );
        for _ in 0..5 {
            press(&mut app, KeyCode::Backspace);
        }
        type_text(&mut app, " new #café");
        let text = save_text(press(&mut app, KeyCode::Enter));
        assert_eq!(
            text,
            "  * [x] (A) old #tag new #café
"
        );
    }

    #[test]
    fn deleting_the_last_task_leaves_the_selection_valid() {
        let (_tmp, mut app) = app(&[(
            "TODOs.md",
            "- [ ] only
",
        )]);
        app.pane = Pane::Todos;
        press(&mut app, KeyCode::Char('d'));
        save_text(press(&mut app, KeyCode::Char('y')));
        assert_eq!(app.task_idx, 0);
        assert!(app.current_task().is_none());
        assert_eq!(press(&mut app, KeyCode::Up), None);
        assert_eq!(press(&mut app, KeyCode::Char('x')), None);
    }

    #[test]
    fn switching_files_resets_the_task_selection_and_keeps_history_per_file() {
        let (_tmp, mut app) = app(&[
            (
                "a/TODOs.md",
                "- [ ] a1
- [ ] a2
",
            ),
            (
                "b/TODOs.md",
                "- [ ] b1
",
            ),
        ]);
        app.pane = Pane::Todos;
        press(&mut app, KeyCode::Down);
        app.on_key(KeyCode::Char('x'));
        assert_eq!(app.history.len(), 1);

        app.pane = Pane::Files;
        press(&mut app, KeyCode::Down);
        assert_eq!(app.file_idx, 1);
        assert_eq!(app.task_idx, 0);
        assert!(app.file_history().is_empty());

        press(&mut app, KeyCode::Up);
        assert_eq!(app.file_idx, 0);
        assert_eq!(app.file_history().len(), 1);
    }

    #[test]
    fn every_edit_appends_a_history_entry_newest_first() {
        let (_tmp, mut app) = app(&[(
            "TODOs.md", "- [ ] a
",
        )]);
        app.pane = Pane::Todos;
        app.on_key(KeyCode::Char('x'));
        app.on_key(KeyCode::Char('B'));
        app.on_key(KeyCode::Char('a'));
        type_text(&mut app, "new");
        app.on_key(KeyCode::Enter);

        let labels: Vec<&str> = app
            .history
            .iter()
            .map(|entry| entry.label.as_str())
            .collect();
        assert_eq!(labels.len(), 3);
        assert_eq!(labels[0], "add · new");
        assert_eq!(labels[1], "priority B · a");
        assert_eq!(labels[2], "check · a");
        assert!(app.history.iter().all(|entry| entry.applied));
    }

    #[test]
    fn x_on_a_history_entry_reverts_it_and_x_again_redoes_it() {
        let (_tmp, mut app) = app(&[(
            "TODOs.md", "- [ ] a
",
        )]);
        app.pane = Pane::Todos;
        app.on_key(KeyCode::Char('x'));
        app.pane = Pane::History;

        let text = save_text(app.on_key(KeyCode::Char('x')));
        assert_eq!(
            text,
            "- [ ] a
"
        );
        assert!(!app.history[0].applied);

        let text = save_text(app.on_key(KeyCode::Char('x')));
        assert_eq!(
            text,
            "- [x] a
"
        );
        assert!(app.history[0].applied);
    }

    #[test]
    fn reverting_a_stale_entry_is_refused_with_a_message() {
        let (_tmp, mut app) = app(&[(
            "TODOs.md", "- [ ] a
",
        )]);
        app.pane = Pane::Todos;
        app.on_key(KeyCode::Char('x'));
        app.pane = Pane::History;

        app.doc = Doc::parse(
            "- [x] someone else
",
        );
        assert_eq!(app.on_key(KeyCode::Char('x')), None);
        assert!(app.history[0].applied);
        let message = app.message.clone().unwrap_or_default();
        assert!(message.contains("press r"), "{message}");
    }

    #[test]
    fn an_external_write_is_picked_up_and_says_so() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("TODOs.md");
        std::fs::write(&path, "- [ ] a\n").unwrap();
        let mut app = crate::App::open(&path).unwrap();

        std::fs::write(&path, "- [ ] a\n- [ ] written elsewhere\n").unwrap();
        app.refresh_from_disk().unwrap();

        assert_eq!(app.doc.render(), "- [ ] a\n- [ ] written elsewhere\n");
        assert!(
            app.message.clone().unwrap_or_default().contains("reloaded"),
            "{:?}",
            app.message
        );
    }

    #[test]
    fn our_own_save_is_not_an_external_write() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("TODOs.md");
        std::fs::write(&path, "- [ ] a\n").unwrap();
        let mut app = crate::App::open(&path).unwrap();
        app.pane = Pane::View;

        let text = save_text(app.on_key(KeyCode::Char('x')));
        let saved = aricma_todo_lib::store::save(&app.baseline, &text).unwrap();
        app.saved(saved);

        app.refresh_from_disk().unwrap();
        assert_eq!(app.doc.render(), "- [x] a\n", "the edit stands");
        assert!(app.history.len() == 1, "and its entry is still there");
    }

    #[test]
    fn a_refresh_waits_while_something_is_being_typed() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("TODOs.md");
        std::fs::write(&path, "- [ ] a\n").unwrap();
        let mut app = crate::App::open(&path).unwrap();
        app.pane = Pane::View;
        app.on_key(KeyCode::Char('a'));
        type_text(&mut app, "half typed");

        std::fs::write(&path, "- [ ] a\n- [ ] elsewhere\n").unwrap();
        app.refresh_from_disk().unwrap();

        assert_eq!(app.doc.render(), "- [ ] a\n", "the document is left alone");
        assert_eq!(
            app.mode,
            Mode::Adding(crate::prompt::Prompt::new("half typed")),
            "and the typing survives"
        );
    }

    #[test]
    fn reload_forgets_the_history() {
        let (_tmp, mut app) = app(&[(
            "TODOs.md", "- [ ] a
",
        )]);
        app.pane = Pane::Todos;
        app.on_key(KeyCode::Char('x'));
        assert_eq!(app.history.len(), 1);
        assert_eq!(app.on_key(KeyCode::Char('r')), Some(Action::Reload));
        app.reload().unwrap();
        assert!(app.history.is_empty());
        assert_eq!(
            app.doc.render(),
            "- [ ] a
"
        );
    }

    #[test]
    fn d_on_a_history_entry_reverts_it_and_forgets_it() {
        let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
        app.pane = Pane::View;
        press(&mut app, KeyCode::Char('x')); // check, which records one entry
        assert_eq!(app.history.len(), 1);

        app.pane = Pane::History;
        let text = save_text(press(&mut app, KeyCode::Char('d')));
        assert_eq!(text, "- [ ] a\n", "the write is undone");
        assert!(app.history.is_empty(), "and the entry is gone");
        assert_eq!(app.mode, Mode::Normal, "no confirmation was asked");
    }

    #[test]
    fn d_on_a_reverted_entry_forgets_it_without_redoing_it() {
        let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
        app.pane = Pane::View;
        press(&mut app, KeyCode::Char('x'));
        app.pane = Pane::History;
        press(&mut app, KeyCode::Char('x')); // uncheck the entry: file is back to [ ]
        assert!(!app.history[0].applied);

        assert_eq!(
            press(&mut app, KeyCode::Char('d')),
            None,
            "nothing to write"
        );
        assert!(app.history.is_empty());
        assert_eq!(app.doc.render(), "- [ ] a\n");
    }

    #[test]
    fn d_on_a_stale_history_entry_keeps_it() {
        let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
        app.pane = Pane::View;
        press(&mut app, KeyCode::Char('x'));
        app.pane = Pane::History;
        app.doc = Doc::parse("- [x] written by someone else\n");

        assert_eq!(press(&mut app, KeyCode::Char('d')), None);
        assert_eq!(
            app.history.len(),
            1,
            "an applied write is not forgotten silently"
        );
        assert!(
            app.message.clone().unwrap_or_default().contains("press r"),
            "{:?}",
            app.message
        );
    }

    #[test]
    fn checking_happens_in_the_view_and_nowhere_else() {
        let (_tmp, mut app) = app(&[("TODOs.md", "# one\n\n- [ ] a\n\n## two\n\n- [ ] b\n")]);
        let source = app.doc.render();

        // the files section is for picking a file, not for checking
        assert_eq!(app.pane, Pane::Files);
        assert_eq!(press(&mut app, KeyCode::Char('x')), None);
        assert_eq!(press(&mut app, KeyCode::Char(' ')), None);
        assert_eq!(app.doc.render(), source);

        // and so is the outline
        press(&mut app, KeyCode::Tab);
        assert_eq!(app.pane, Pane::Outline);
        assert_eq!(press(&mut app, KeyCode::Char('x')), None);
        assert_eq!(app.doc.render(), source);

        // the view is where a todo is checkable
        press(&mut app, KeyCode::Right);
        assert_eq!(app.pane, Pane::View);
        let text = save_text(press(&mut app, KeyCode::Char('x')));
        assert_eq!(text, "# one\n\n- [x] a\n\n## two\n\n- [ ] b\n");
    }

    #[test]
    fn there_are_no_vim_motions() {
        let (_tmp, mut app) = app(&[("TODOs.md", "# one\n\n- [ ] a\n\n## two\n\n- [ ] b\n")]);
        for key in "jklh".chars().map(KeyCode::Char) {
            assert_eq!(app.on_key(key), None, "{key:?} must do nothing");
        }
        assert_eq!(app.pane, Pane::Files, "still on the first section");
        assert_eq!(app.current_task().unwrap().title, "a");

        // the arrows do the moving instead: down is "inside this panel", so the
        // todos are walked once the arrows have crossed into the view
        press(&mut app, KeyCode::Right);
        assert_eq!(app.pane, Pane::View);
        press(&mut app, KeyCode::Down);
        assert_eq!(app.current_task().unwrap().title, "b");
        press(&mut app, KeyCode::Up);
        assert_eq!(app.current_task().unwrap().title, "a");
    }

    #[test]
    fn the_files_section_does_not_touch_a_todo_but_still_adds_one() {
        let (_tmp, mut app) = app(&[("TODOs.md", "- [ ] a\n")]);
        assert_eq!(app.pane, Pane::Files);
        assert_eq!(
            press(&mut app, KeyCode::Char('x')),
            None,
            "no checking here"
        );
        assert_eq!(app.doc.render(), "- [ ] a\n");

        // adding is a different key and still works from the file list
        press(&mut app, KeyCode::Char('a'));
        assert_eq!(app.mode, Mode::Adding(crate::prompt::Prompt::new("")));
    }

    #[test]
    fn q_quits_and_saves_carry_the_open_path() {
        let (tmp, mut app) = app(&[(
            "TODOs.md", "- [ ] a
",
        )]);
        app.pane = Pane::Todos;
        assert_eq!(
            app.on_key(KeyCode::Char('x')),
            Some(Action::Save {
                path: tmp.path().join("TODOs.md"),
                text: "- [x] a
"
                .to_owned()
            })
        );
        assert_eq!(app.on_key(KeyCode::Char('q')), Some(Action::Quit));
    }
}
