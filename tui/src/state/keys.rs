//! Keys to state: the whole key map in one place.

use aricma_todo_lib::error::Error;
use aricma_todo_lib::model::{Blocker, Inverse, Prio, Task};

use super::label::label;
use super::retrace::retrace;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::action::{Action, Pane};
use super::app::App;
use super::confirm::Confirm;
use super::create::Create;
use super::prompt::{Mode, Prompt};
use super::row::Row;
use crate::helpers::shift;
use crate::text;

/// What the line being typed will become when it is committed.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Typing {
    NewTodo,
    NewHeading,
    EditTodo,
}

impl App {
    /// Handle one key press — the only way in the terminal layer has.
    ///
    /// A press either changes the state or returns an `Action` for the caller to
    /// perform. Creating or deleting a file is the one thing done here rather than
    /// returned, because what is on screen next depends on how it went.
    pub fn on_key(&mut self, key: KeyEvent) -> Option<Action> {
        self.last_key = Some(key);
        // a hint lives until the next key press: nothing is carried around after
        // the thing it was about stopped being true
        self.clear_message();
        // The modified forms of the search key still work where a terminal does
        // forward them, and `cmd+f` is the one most of them take for their own
        // find. Quitting is the one key that works in every mode, including a
        // field, where the letters below are letters.
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::SUPER)
        {
            match key.code {
                KeyCode::Char('c') => return Some(Action::Quit),
                KeyCode::Char('f') => {
                    self.open_search();
                    return None;
                }
                KeyCode::Char('u') => {
                    self.clear_search();
                    return None;
                }
                _ => {}
            }
        }
        // A modified arrow does one of three things, and which modifier arrives
        // is the terminal's business — opt is `alt`, `meta` or `super` depending
        // on where you are — so the rule names what it *does*:
        //
        //   shift  the first or last row of this section
        //   ctrl   move the todo up or down
        //   opt    the next or previous sidebar section, where there is a sidebar
        //   any    move the todo, which is what a sideways arrow cannot do
        if !key.modifiers.is_empty() {
            if let KeyCode::Up | KeyCode::Down = key.code {
                let up = key.code == KeyCode::Up;
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    return self.jump(!up);
                }
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    return self.move_task(up);
                }
                // the file view has no sidebar to step through, so opt keeps its
                // old meaning where it has no new one
                if self.pane.in_sidebar() {
                    self.focus(!up);
                    return None;
                }
                return self.move_task(up);
            }
            // every other modified key is the key it is: a terminal reports
            // `shift+tab` as a modifier on the same key
        }
        self.press(key.code)
    }

    /// One unmodified key, by its code.
    pub fn press(&mut self, code: KeyCode) -> Option<Action> {
        match self.mode.clone() {
            Mode::Searching => self.searching(code),
            Mode::Adding(prompt) => self.typing(prompt, Typing::NewTodo, code),
            Mode::AddingHeading(prompt) => self.typing(prompt, Typing::NewHeading, code),
            Mode::Editing(prompt) => self.typing(prompt, Typing::EditTodo, code),
            Mode::Creating(create) => self.creating(create, code),
            Mode::Help => self.helping(code),
            Mode::Confirming(confirm) => self.confirming(confirm, code),
            Mode::Conflicted => self.conflicted(code),
            Mode::Normal => self.normal(code),
        }
    }

    /// `?`: every binding, grouped by the panel it works in.
    fn help(&mut self) {
        self.mode = Mode::Help;
    }

    /// A key while the keys page is open: `?` and `esc` close it, `q` quits the
    /// app like it does everywhere else, and nothing else does anything — a page
    /// about keys should not be a place to press keys.
    fn helping(&mut self, code: KeyCode) -> Option<Action> {
        match code {
            KeyCode::Esc | KeyCode::Char('?') => self.mode = Mode::Normal,
            KeyCode::Char('q') => return Some(Action::Quit),
            _ => {}
        }
        None
    }

    /// A key while the new-file field is open: type, mark a folder, create, cancel.
    fn creating(&mut self, mut create: Create, code: KeyCode) -> Option<Action> {
        match code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                return None;
            }
            KeyCode::Enter => {
                // the field blocks enter itself when it says why the path is no
                // good, so a wrong action cannot be taken
                if create.refusal.is_some() {
                    return None;
                }
                self.mode = Mode::Normal;
                return self.create(&create);
            }
            KeyCode::Up => create.mark(-1),
            KeyCode::Down => create.mark(1),
            KeyCode::Left => create.prompt_mut().left(),
            KeyCode::Right => create.prompt_mut().right(),
            KeyCode::Home => create.prompt_mut().home(),
            KeyCode::End => create.prompt_mut().end(),
            KeyCode::Backspace => create.prompt_mut().backspace(),
            KeyCode::Delete => create.prompt_mut().delete(),
            KeyCode::Char(character) => create.accept(character),
            _ => {}
        }
        create.refresh();
        self.mode = Mode::Creating(create);
        None
    }

    /// Make the folder and its file, then put the cursor on it.
    fn create(&mut self, create: &Create) -> Option<Action> {
        match self.create_file(&create.folder()) {
            Ok(()) => None,
            // the field is gone, so this one has to be said under the file list
            Err(error) => {
                self.set_error(error.to_string());
                None
            }
        }
    }

    /// A key while the search popover is open: type, pick, jump, close.
    fn searching(&mut self, code: KeyCode) -> Option<Action> {
        match code {
            KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Enter => return self.jump_to_hit(),
            KeyCode::Up => self.search.select(-1),
            KeyCode::Down => self.search.select(1),
            KeyCode::Left => self.search.query.left(),
            KeyCode::Right => self.search.query.right(),
            KeyCode::Home => self.search.query.home(),
            KeyCode::End => self.search.query.end(),
            KeyCode::Backspace => {
                self.search.query.backspace();
                self.search.refresh(&self.files);
            }
            KeyCode::Char(character) => {
                self.search.query.insert(character);
                self.search.refresh(&self.files);
            }
            _ => {}
        }
        None
    }

    /// `ctrl+f`: open the popover, which re-runs the query it kept. One modal at a
    /// time: it never opens on top of another one, so a half-typed field cannot be
    /// stranded behind it.
    pub(crate) fn open_search(&mut self) {
        if self.modal() {
            return;
        }
        self.mode = Mode::Searching;
        self.search.refresh(&self.files);
    }

    /// `ctrl+u`: empty the query. It cannot be `x` — this is a text field, and a
    /// key that erases what you typed makes searching for "xfce" impossible.
    /// Nothing to clear while the popover is closed.
    pub(crate) fn clear_search(&mut self) {
        if self.mode == Mode::Searching {
            self.search.clear(&self.files);
        }
    }

    /// Enter on a hit: open its file, select its todo, close the popover.
    fn jump_to_hit(&mut self) -> Option<Action> {
        let hit = self.search.current().cloned()?;
        // a hit names a file the scan knows; anything else is not ours to jump to
        self.files.iter().position(|file| file.path == hit.path)?;
        self.mode = Mode::Normal;
        // the todo is what the search was for, so it is what the view shows
        if let Err(error) = self.jump_to(&hit.path, hit.line) {
            self.set_warning(error.to_string());
        }
        None
    }

    /// A key while a prompt is open: type, move, commit, cancel.
    fn typing(&mut self, mut prompt: Prompt, what: Typing, code: KeyCode) -> Option<Action> {
        match code {
            KeyCode::Esc => {
                if what == Typing::NewTodo {
                    self.drop_placeholder();
                }
                self.mode = Mode::Normal;
                return None;
            }
            KeyCode::Enter => {
                let text = prompt.text().to_owned();
                self.mode = Mode::Normal;
                let action = match what {
                    Typing::NewTodo => self.add(&text),
                    Typing::NewHeading => self.add_heading(&text),
                    Typing::EditTodo => self.edit(&text),
                };
                if action.is_some() {
                    return action;
                }
                // a line the rules refuse keeps the field open with what was typed:
                // throwing it away is the worse answer
                match what {
                    Typing::NewTodo => {
                        self.set_warning(text::NEEDS_A_TITLE);
                        self.mode = Mode::Adding(Prompt::new(&text));
                    }
                    Typing::NewHeading => {
                        self.set_warning(text::NEEDS_A_HEADING);
                        self.mode = Mode::AddingHeading(Prompt::new(&text));
                    }
                    Typing::EditTodo => {}
                }
                return None;
            }
            KeyCode::Left => prompt.left(),
            KeyCode::Right => prompt.right(),
            KeyCode::Home => prompt.home(),
            KeyCode::End => prompt.end(),
            KeyCode::Backspace => prompt.backspace(),
            KeyCode::Delete => prompt.delete(),
            KeyCode::Char(character) => prompt.insert(character),
            _ => {}
        }
        self.mode = match what {
            Typing::NewTodo => Mode::Adding(prompt),
            Typing::NewHeading => Mode::AddingHeading(prompt),
            Typing::EditTodo => Mode::Editing(prompt),
        };
        None
    }

    /// `n` in the outline: a heading for this file, at the end of it.
    fn add_heading(&mut self, text: &str) -> Option<Action> {
        let inverse = self.doc.add_heading(text)?;
        // the new heading is the last one, so the cursor lands on what was just
        // written
        self.outline_idx = self.headings().len().saturating_sub(1);
        let subject = text.trim_start_matches('#').trim();
        Some(self.record(format!("heading · {subject}"), inverse))
    }

    /// A save hit a file that changed under it, and the two versions cannot both
    /// win.
    ///
    /// Nothing dismisses this by accident: cancelling would have to pick a side.
    /// `Y` keeps ours, `t` takes the file's, `q` quits, and every other key is
    /// ignored — the question waits.
    fn conflicted(&mut self, code: KeyCode) -> Option<Action> {
        match code {
            KeyCode::Char('Y') => {
                self.mode = Mode::Normal;
                self.set_info(text::kept_mine());
                Some(Action::Overwrite {
                    text: self.doc.render(),
                })
            }
            KeyCode::Char('t') => {
                match self.take_theirs() {
                    Ok(()) => {}
                    Err(error) => self.set_error(error.to_string()),
                }
                self.mode = Mode::Normal;
                None
            }
            KeyCode::Char('q') => Some(Action::Quit),
            _ => None,
        }
    }

    /// A delete question: it is answered by **one** key.
    ///
    /// Any other key dismisses it and nothing is deleted — so no key pressed while
    /// a question is open can destroy something by accident, and `esc` is not a key
    /// a person has to remember. `q` still quits the app, as it does everywhere
    /// else, and the delete it interrupts does not happen.
    fn confirming(&mut self, confirm: Confirm, code: KeyCode) -> Option<Action> {
        self.mode = Mode::Normal;
        if code == KeyCode::Char('q') {
            return Some(Action::Quit);
        }
        if code != KeyCode::Char('Y') {
            return None;
        }
        match confirm {
            Confirm::Todo => {
                let task = self.current_task()?;
                let inverse = self.doc.delete(task.line)?;
                self.clamp_task();
                Some(self.record(label(text::DELETE_TODO, &task), inverse))
            }
            Confirm::File => {
                let path = self.current().path.clone();
                match self.delete_file(&path) {
                    Ok(()) => None,
                    Err(error) => {
                        self.set_error(error.to_string());
                        None
                    }
                }
            }
        }
    }

    fn normal(&mut self, code: KeyCode) -> Option<Action> {
        match code {
            KeyCode::Char('q') => Some(Action::Quit),
            // tab loops the sidebar; the sideways arrows cross to the file view
            KeyCode::Tab => {
                self.focus(true);
                None
            }
            KeyCode::BackTab => {
                self.focus(false);
                None
            }
            KeyCode::Right => {
                // a collection row reads its todos; a file row opens the file view
                if let Err(error) = self.enter_view() {
                    self.set_warning(error.to_string());
                }
                None
            }
            KeyCode::Left => {
                self.leave_view();
                None
            }
            // `enter` takes you to what the row is about: a todo in a collection is
            // a line in a file, and this is how you get to that file with the cursor
            // on it. It is the promise `enter` already makes in the search popover
            KeyCode::Enter if self.pane == Pane::Collection => {
                if let Some(item) = self.items.get(self.task_idx).cloned() {
                    if let Err(error) = self.jump_to(&item.path, item.line) {
                        self.set_warning(error.to_string());
                    }
                }
                None
            }
            KeyCode::Down => self.step(1),
            KeyCode::Up => self.step(-1),
            // the ends of a section have three spellings on purpose: `g`/`G` are
            // letters, `home`/`end` are keys every terminal sends, and
            // `shift+↑/↓` is the one that only arrives where the terminal reports
            // modifiers at all (see "Terminals that eat the modifier")
            KeyCode::Char('g') | KeyCode::Home => self.jump(false),
            KeyCode::Char('G') | KeyCode::End => self.jump(true),
            KeyCode::Char('x' | ' ') => self.toggle(),
            // the plain fallback: a terminal that eats the arrow's modifier still
            // sends a letter, and `K`/`J` sit beside the cursor keys
            KeyCode::Char('K') => self.move_task(true),
            KeyCode::Char('J') => self.move_task(false),
            KeyCode::Char('d') => self.ask_delete(),
            // `n` is new: a todo where todos are edited, a file where files are
            // chosen. One key for one idea.
            KeyCode::Char('n') => match self.pane {
                Pane::Files => {
                    // a new file goes in the folder of the file row you are on
                    if self.selected_file().is_some() {
                        self.ask_create()
                    } else {
                        None
                    }
                }
                Pane::View | Pane::Todos => self.ask_add(),
                Pane::Outline => self.ask_heading(),
                // a collection is a view of files: a new todo has no file to go in
                Pane::Collection | Pane::History => None,
            },
            // `f` finds: a plain letter, because the terminals that matter keep
            // `cmd+f` for their own search and a key nobody can press is not a key
            KeyCode::Char('f') => {
                self.open_search();
                None
            }
            KeyCode::Char('?') => {
                self.help();
                None
            }
            // roomy or tight: a blank row between rows. A terminal has no
            // half-line, so this is the whole of "line height" and it costs
            // density — which is why it is a switch and not a constant
            // `c` copies: the path of the file, or the line the todo is
            KeyCode::Char('c') => self.copy(),
            KeyCode::Char('t') => {
                self.roomy = !self.roomy;
                None
            }
            // `h` hides what the convention hides: every markdown file in the tree,
            // or only the ones named for todos. A team that calls its file
            // `backlog.md` keeps its todos where the convention says it may not.
            // It is the file panel's key, on a file row or a collection row: it
            // changes what the *scan* found, and the collection panel is reading
            // todos rather than looking for files. A previewed collection is
            // gathered again, or it would keep showing todos from a hidden file
            KeyCode::Char('h') if self.pane == Pane::Files => {
                self.show_all = !self.show_all;
                let focus = self.current().path.clone();
                if let Err(error) = self.rescan(&focus) {
                    self.set_warning(error.to_string());
                    return None;
                }
                if let Some(collection) = self.collection.clone() {
                    if let Err(error) = self.load_collection(&collection) {
                        self.set_warning(error.to_string());
                    }
                }
                None
            }
            // `v` cycles what the file panel shows: files, then tags, then people
            KeyCode::Char('v') => {
                self.cycle_showing();
                None
            }
            KeyCode::Char('e') => self.ask_edit(),
            KeyCode::Char('0') => self.priority(None),
            KeyCode::Char(letter @ 'A'..='D') => {
                let prio = Prio::from_letter(letter)?;
                self.priority(Some(prio))
            }
            KeyCode::Esc => {
                self.clear_message();
                None
            }
            _ => None,
        }
    }

    /// Move the focus to the next or previous sidebar section, looping. From the
    /// file view, `tab` goes back to the sidebar rather than staying on the right.
    pub fn focus(&mut self, forward: bool) {
        if let Some(pane) = self.focus_target(forward) {
            self.pane = pane;
        }
    }

    /// Move one row inside the focused section.
    fn step(&mut self, delta: isize) -> Option<Action> {
        match self.pane {
            Pane::Files => {
                if let Err(error) = self.select_file(delta) {
                    self.set_warning(error.to_string());
                }
                self.sync_outline();
            }
            Pane::Outline => {
                self.outline_idx = shift(self.outline_idx, delta, self.headings().len());
                self.jump_from_outline();
            }
            Pane::View | Pane::Todos | Pane::Collection => {
                self.todo(delta);
            }
            Pane::History => {
                self.history_idx = shift(self.history_idx, delta, self.file_history().len());
            }
        }
        None
    }

    /// Move the selected todo, and take the outline and the open file with it.
    ///
    /// In a collection the cursor walks the gathered todos; the file that is open
    /// follows the one you land on, because that is the file every edit and the
    /// history panel belong to.
    fn todo(&mut self, delta: isize) -> Option<Action> {
        let count = if self.collection.is_some() {
            self.items.len()
        } else {
            self.tasks().len()
        };
        self.task_idx = shift(self.task_idx, delta, count);
        if let Err(error) = self.follow_item() {
            self.set_warning(error.to_string());
        }
        self.sync_outline();
        None
    }

    /// Open the file the highlighted item of a collection came from, so the document
    /// under the cursor is the one an edit would write to.
    fn follow_item(&mut self) -> Result<(), Error> {
        if self.collection.is_none() {
            return Ok(());
        }
        let Some(item) = self.items.get(self.task_idx) else {
            return Ok(());
        };
        if item.path == *self.current().path {
            return Ok(());
        }
        let Some(index) = self.files.iter().position(|file| file.path == item.path) else {
            return Ok(());
        };
        self.file_idx = index;
        // `show` puts the cursor back at the top of the document, which is right for
        // opening a file and wrong here: the collection's place is the todo
        let at = self.task_idx;
        self.open_current()?;
        self.task_idx = at;
        Ok(())
    }

    fn jump(&mut self, last: bool) -> Option<Action> {
        match self.pane {
            Pane::Files => {
                let target = if last {
                    self.rows.len().saturating_sub(1)
                } else {
                    0
                };
                // the cursor's row, not the open file's: the collections below the
                // files are rows the cursor can stand on, and `g` from one of them
                // still means the first row
                let delta = i64::try_from(target).unwrap_or(i64::MAX)
                    - i64::try_from(self.row_idx).unwrap_or(0);
                let delta = isize::try_from(delta).unwrap_or(isize::MAX);
                if let Err(error) = self.select_file(delta) {
                    self.set_warning(error.to_string());
                }
            }
            Pane::Outline => {
                let count = self.headings().len();
                self.outline_idx = if last { count.saturating_sub(1) } else { 0 };
                self.jump_from_outline();
            }
            Pane::View | Pane::Todos | Pane::Collection => {
                let count = if self.collection.is_some() {
                    self.items.len()
                } else {
                    self.tasks().len()
                };
                self.task_idx = if last { count.saturating_sub(1) } else { 0 };
                if let Err(error) = self.follow_item() {
                    self.set_warning(error.to_string());
                }
                self.sync_outline();
            }
            Pane::History => {
                let count = self.file_history().len();
                self.history_idx = if last { count.saturating_sub(1) } else { 0 };
            }
        }
        None
    }

    /// `x`: check a todo, or uncheck a history entry.
    ///
    /// Checking belongs to the panel that shows the todo: the file view (or the
    /// todos list of a single-file session) and nowhere else. In the files and
    /// outline sections `x` does nothing, so a key pressed while picking a file
    /// can never change a todo.
    fn toggle(&mut self) -> Option<Action> {
        // a comment has no checkbox to flip, and its only "done" is being deleted —
        // or copied into a todo file to be tracked there
        if self.inline() {
            self.set_warning(text::INLINE_NO_CHECK);
            return None;
        }
        match self.pane {
            Pane::History => self.toggle_entry(),
            Pane::View | Pane::Todos | Pane::Collection => {
                let task = self.current_task()?;
                let inverse = self.doc.check(task.line)?;
                let after = self.task_at(task.line)?;
                let action = if after.done {
                    text::CHECK_TODO
                } else {
                    text::UNCHECK_TODO
                };
                Some(self.record(label(action, &after), inverse))
            }
            Pane::Files | Pane::Outline => None,
        }
    }

    /// Uncheck a history entry to undo its write; check it again to redo it.
    fn toggle_entry(&mut self) -> Option<Action> {
        let global = *self.file_history().get(self.history_idx)?;
        let entry = self.history.get(global)?.clone();
        if entry.applied {
            // undoing a write takes the writes that hang on it — and only those:
            // a write that changed something else on the line stays applied
            let mut step = self.dependents(global);
            step.push(global);
            if !self.undo(&step) {
                return None;
            }
            self.follow(&entry.forward);
            return Some(self.save_action());
        }
        // and a write can only go back on top of the one it undoes; the row says
        // so by itself while it is selected
        if self
            .dependency(global)
            .is_some_and(|under| !self.history[under].applied)
        {
            return None;
        }
        let mut doc = self.doc.clone();
        match retrace(&mut doc, &entry.forward, false) {
            Ok(applied) => {
                self.doc = doc;
                // a redo applies the write itself — only an undo applies its flip
                self.history[global].forward = applied.clone();
                self.history[global].applied = true;
                self.refresh_stats();
                self.follow(&applied);
                Some(self.save_action())
            }
            Err(reason) => {
                self.warn_on(global, format!("{reason} — {}", text::IT_RELOADS));
                None
            }
        }
    }

    /// `c`: put the selected thing on the clipboard.
    ///
    /// A file gives its path and a heading gives its text, because those are strings
    /// and a person uses them as strings. A **todo** gives an OPS document — title,
    /// status, priority, tags, people — because a todo is structured data: pasted back
    /// into a todos file it is drawn as a line, pasted into anything else it is the
    /// same todo, and an OPS document from elsewhere pastes the same way (RFC-014).
    /// A collection row gives the word it is — `#home`, `@sam` — for a search or a new
    /// todo, which is what a collection *is*.
    fn copy(&mut self) -> Option<Action> {
        let (text, what) = match self.pane {
            // the row says what it is: a collection row is a name, not a file
            Pane::Files => {
                if let Some(Row::Collection(collection)) = self.selected_row() {
                    let word = collection.to_string();
                    (word.clone(), word)
                } else {
                    let path = self.current().path.clone();
                    let shown = self.display(&path);
                    (path.display().to_string(), shown)
                }
            }
            Pane::View | Pane::Todos | Pane::Collection => {
                let task = self.current_task()?;
                // the label is the line, which is what the person recognises; the
                // clipboard gets the document
                let line = self.doc.lines().get(task.line)?.clone();
                match aricma_todo_lib::ops::one(&task) {
                    Ok(json) => (json, line),
                    // a todo with no title has nothing to be a task about: say so
                    // rather than quietly putting the markdown on the clipboard, and
                    // leaving a paste to guess
                    Err(error) => {
                        self.set_warning(error.to_string());
                        return None;
                    }
                }
            }
            // a heading is one line of text too
            Pane::Outline => {
                let heading = self.headings().get(self.outline_idx)?.text.clone();
                (heading.clone(), heading)
            }
            Pane::History => return None,
        };
        self.message = Some(crate::state::message::Message::success(text::copied(&what)));
        Some(Action::Copy { text })
    }

    /// What arrived from the clipboard: `Event::Paste`, one string.
    ///
    /// In a field, a paste is text and goes in at the cursor — a path, a query, a
    /// sentence of a todo. Anywhere else it is a **todo** being pasted, so it is read
    /// as an OPS document and written out as todo lines under the highlighted one: the
    /// file gets markdown, the person gets their todos, and a document copied out of
    /// another tool needs no conversion (RFC-014).
    pub fn on_paste(&mut self, text: &str) -> Option<Action> {
        if self.mode != Mode::Normal {
            // a text field takes text, whatever it is
            let mut action = None;
            for character in text.chars().filter(|one| !one.is_control()) {
                if let Some(one) = self.press(KeyCode::Char(character)) {
                    action = Some(one);
                }
            }
            return action;
        }
        if !matches!(self.pane, Pane::View | Pane::Todos) || self.inline() {
            self.set_warning(text::PASTE_NOT_HERE);
            return None;
        }
        let after = self
            .current_task()
            .map_or_else(|| self.doc.lines().len(), |task| task.line + 1);
        let written = match aricma_todo_lib::ops::paste(&mut self.doc, text, after) {
            Ok(written) => written,
            Err(error) => {
                let _ = error;
                self.set_warning(text::NOT_OPS);
                return None;
            }
        };
        let Some(&first) = written.first() else {
            self.set_warning(text::NO_TODOS_IN_OPS);
            return None;
        };
        // one entry per line: the history is per line, and undo peels the paste from
        // the bottom up, which is how a paste into a file someone cares about is safe
        for line in &written {
            let text = self.doc.lines().get(*line)?.clone();
            let task = self.task_at(*line)?;
            self.record(label(text::NEW_TODO, &task), Inverse::Insert { line: *line, text });
        }
        self.task_idx = self
            .tasks()
            .iter()
            .position(|task| task.line == first)
            .unwrap_or(0);
        self.set_info(text::pasted(written.len()));
        Some(self.save_action())
    }

    /// A move leaves the todo at the other end of its swap, or where it landed when
    /// it crossed a heading; keep the cursor on it.
    fn follow(&mut self, inverse: &Inverse) {
        let landed = match inverse {
            Inverse::Swap { a, b, .. } => {
                let Some(task) = self.current_task() else {
                    return;
                };
                if task.line == *a { *b } else { *a }
            }
            Inverse::Move { to, .. } => *to,
            _ => return,
        };
        if let Some(index) = self.tasks().iter().position(|task| task.line == landed) {
            self.task_idx = index;
        }
    }

    /// Put these writes back, newest first, on a copy of the document: a write
    /// whose line moved stops the whole step, so nothing ends up half undone.
    fn undo(&mut self, step: &[usize]) -> bool {
        let mut doc = self.doc.clone();
        let mut done = Vec::new();
        for index in step {
            if !self.history[*index].applied {
                continue;
            }
            let forward = self.history[*index].forward.clone();
            match retrace(&mut doc, &forward, true) {
                Ok(inverse) => done.push((*index, inverse)),
                Err(reason) => {
                    self.warn_on(*index, format!("{reason} — {}", text::IT_RELOADS));
                    return false;
                }
            }
        }
        self.doc = doc;
        for (index, inverse) in done {
            self.history[index].forward = inverse.flip();
            self.history[index].applied = false;
        }
        self.refresh_stats();
        true
    }

    /// `d`: delete the selected todo, the selected file, or forget the selected
    /// history entry.
    ///
    /// Deleting a todo asks first, because the line is gone afterwards. Forgetting
    /// a history entry does not: the write it undoes is being reverted at the same
    /// time, and the entry itself is what was being deleted, so there is nothing
    /// left to ask about and nothing to record.
    fn ask_delete(&mut self) -> Option<Action> {
        match self.pane {
            Pane::History => self.forget_entry(),
            Pane::View | Pane::Todos | Pane::Collection => {
                if self.current_task().is_some() {
                    self.mode = Mode::Confirming(Confirm::Todo);
                }
                None
            }
            Pane::Files => self.ask_delete_file(),
            Pane::Outline => None,
        }
    }

    /// `d` in the file panel: remove the file, always after asking.
    ///
    /// There used to be an exception for a file of zero bytes. A file the tool
    /// creates is born with its heading now, and the scan only lists files that
    /// hold a todo, so a listed file always has something in it — one rule instead
    /// of two.
    fn ask_delete_file(&mut self) -> Option<Action> {
        // only a file row names a file to delete
        self.selected_file()?;
        self.mode = Mode::Confirming(Confirm::File);
        None
    }

    /// Undo a history entry's write and drop the entry. A refused revert leaves
    /// both alone, because an applied write is not something to forget silently.
    fn forget_entry(&mut self) -> Option<Action> {
        let global = *self.file_history().get(self.history_idx)?;
        let applied = self.history.get(global)?.applied;
        let mut wrote = false;
        if applied {
            // forgetting a write means undoing it, and its dependents go with it
            let mut step = self.dependents(global);
            step.push(global);
            if !self.undo(&step) {
                return None;
            }
            wrote = true;
        }
        self.history.remove(global);
        self.history_idx = self
            .history_idx
            .min(self.file_history().len().saturating_sub(1));
        if wrote {
            Some(self.save_action())
        } else {
            None
        }
    }

    /// `n` in the file panel: a folder for a new todo file, starting from the
    /// selected file's own folder — that is where a person adding a file is already
    /// looking.
    fn ask_create(&mut self) -> Option<Action> {
        let start = self.selected_folder();
        self.mode = Mode::Creating(Create::new(&self.root, &start));
        None
    }

    /// `n` in a todo panel: an empty prompt for a new todo.
    /// `n`: the empty row goes in first, the highlight moves onto it, and the field
    /// opens over the screen waiting for what goes in it.
    ///
    /// The row is the answer to "where does this land", and the field is the question:
    /// the person sees the todo's place before they decide its words. Nothing reaches
    /// the disk until `enter`; `esc` takes the row out and puts the highlight back
    /// where it was.
    fn ask_add(&mut self) -> Option<Action> {
        // an inline file's todo is a comment in the code: where it goes is the
        // person's decision and the line is written by hand (RFC-016)
        if self.inline() {
            self.set_warning(text::INLINE_NO_ADD);
            return None;
        }
        let after = self.current_task().map(|task| task.line);
        let before = self.task_idx;
        let line = self.doc.add_placeholder(after)?;
        self.task_idx = self
            .tasks()
            .iter()
            .position(|task| task.line == line)
            .unwrap_or(0);
        self.add = Some((line, before));
        self.mode = Mode::Adding(Prompt::new(""));
        None
    }

    /// Put the row a new todo was waiting in back the way it was: the field was
    /// cancelled, and a line nobody typed is not part of the file.
    fn drop_placeholder(&mut self) {
        let Some((line, before)) = self.add.take() else {
            return;
        };
        // never recorded in the history, so its inverse is not kept either
        self.doc.delete(line);
        // and the cursor goes back to the row it came from
        self.task_idx = before.min(self.tasks().len().saturating_sub(1));
    }

    /// `n` in the outline: an empty prompt for a new heading.
    fn ask_heading(&mut self) -> Option<Action> {
        self.mode = Mode::AddingHeading(Prompt::new(""));
        None
    }

    /// `e`: the selected todo's body, ready to edit.
    fn ask_edit(&mut self) -> Option<Action> {
        let task = self.current_task()?;
        self.mode = Mode::Editing(Prompt::new(&task.body));
        None
    }

    fn add(&mut self, body: &str) -> Option<Action> {
        let (line, _) = self.add?;
        // the row is already in the file; this fills it in. The entry is the line
        // being *there*, not the text being set: undoing "new todo" takes the line
        // out, and redoing puts that same line back
        self.doc.set_body(line, body)?;
        self.add = None;
        let done = self.task_at(line)?;
        let entry = Inverse::Insert {
            line,
            text: self.doc.lines().get(line)?.clone(),
        };
        Some(self.record(label(text::NEW_TODO, &done), entry))
    }

    /// `opt+↑` / `opt+↓` (or `K` / `J`): swap the selected todo with the nearest one
    /// above or below it, staying inside the section.
    fn move_task(&mut self, up: bool) -> Option<Action> {
        // a collection's order is its files' order, so there is nothing to move
        if self.collection.is_some() {
            self.set_warning(text::NO_MOVING);
            return None;
        }
        // a comment's line is where the code needs it: moving it would move the
        // note away from the thing it is about
        if self.inline() {
            self.set_warning(text::INLINE_NO_MOVE);
            return None;
        }
        let line = self.current_task()?.line;
        let inverse = match self.doc.move_task(line, up) {
            Ok(inverse) => inverse,
            Err(Blocker::Edge) => {
                // a move that cannot happen says why: silence here reads as "the
                // key does not work"
                let way = if up { "above" } else { "below" };
                self.set_warning(text::nothing_to_swap(way));
                return None;
            }
        };
        self.follow(&inverse);
        let moved = self.current_task()?;
        Some(self.record(label(text::MOVE_TODO, &moved), inverse))
    }

    fn edit(&mut self, body: &str) -> Option<Action> {
        let task = self.current_task()?;
        let inverse = self.doc.set_body(task.line, body)?;
        let after = self.task_at(task.line)?;
        Some(self.record(label(text::EDIT_TODO, &after), inverse))
    }

    fn priority(&mut self, prio: Option<Prio>) -> Option<Action> {
        let task = self.current_task()?;
        // the letter a todo already carries clears it: () -A-> (A) -A-> ()
        let prio = match prio {
            Some(prio) if Some(prio) == task.prio => None,
            other => other,
        };
        let inverse = self.doc.set_prio(task.line, prio)?;
        let after = self.task_at(task.line)?;
        let action = match prio {
            Some(prio) => text::set_prio(prio.letter()),
            None => text::CLEAR_PRIO.to_owned(),
        };
        Some(self.record(label(&action, &after), inverse))
    }

    /// One task by its line, read back after an edit.
    fn task_at(&self, line: usize) -> Option<Task> {
        self.tasks().into_iter().find(|task| task.line == line)
    }
}
