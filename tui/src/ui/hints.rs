//! The two rows of the status bar: the keys every panel shares, then the keys of
//! whatever has the keyboard right now.

use super::consts::SHORT_PATH;
use crate::helpers::shorten;
use crate::state::action::Pane;
use crate::state::confirm::Confirm;
use crate::state::prompt::Mode;
use crate::state::row::Row;
use crate::state::App;
use crate::text;

/// What a modal adds, since a modal owns the keyboard while it is open. `None`
/// when no modal is open and the panel's own keys are the answer.
///
/// The app, not just the mode, because a modal's keys are not all always available:
/// a search with no hit has nothing for `enter` to open.
pub fn modal_keys(app: &App) -> Option<Vec<String>> {
    let words = |keys: &[&str]| keys.iter().map(|key| (*key).to_owned()).collect();
    let keys: Vec<String> = match &app.mode {
        Mode::Normal => return None,
        // the cancel first, then the key that commits: the same order the
        // popup's own border uses
        Mode::Adding(_) | Mode::AddingHeading(_) => words(&[text::ESC_CANCEL, text::ENTER_ADD]),
        Mode::Editing(_) => words(&[text::ESC_CANCEL, text::ENTER_SAVE]),
        Mode::Creating(_) => words(&[text::ESC_CANCEL, text::ENTER_CREATE]),
        Mode::Help => words(&[text::ESC_CLOSE, text::Q_QUIT]),
        // no dismiss key on purpose: cancelling a conflict would have to pick a side
        Mode::Conflicted => words(&[text::KEEP_MINE, text::TAKE_THEIRS]),
        // the same words as the popover's own border: it says "cancel" because the
        // query is a thing you typed and walking away from it is a decision, and
        // `enter` is left out when the query found nothing to open
        Mode::Searching => {
            let mut keys = words(&[text::ESC_CANCEL]);
            if app.search.current().is_some() {
                keys.push(text::ENTER_JUMP.to_owned());
            }
            keys.extend(words(&[text::PICK, text::CLEAR_SEARCH]));
            keys
        }
        Mode::Confirming(confirm) => match &confirm {
            // one rule for both: any key cancels, only a capital Y deletes
            Confirm::Todo | Confirm::File => words(&[text::ESC_CANCEL, text::Y_DELETE]),
        },
    };
    Some(keys)
}

/// Where a binding works, as bits.
///
/// `ANY` is zero and means every panel: the keys that move you around work wherever
/// you are. Every other value is a requirement — at least one of its bits has to be
/// true of the app right now.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Where(u8);

impl Where {
    pub const ANY: Self = Self(0);
    pub const FILES: Self = Self(1 << 0);
    /// a file row: the rows below the files are collections, and a collection can be
    /// neither made nor deleted
    pub const FILE_ROW: Self = Self(1 << 1);
    pub const OUTLINE: Self = Self(1 << 2);
    /// a panel showing the open file's todos: the view and the todos list
    pub const TODO: Self = Self(1 << 3);
    /// the todos a collection gathered, which live in other files
    pub const COLLECTION: Self = Self(1 << 4);
    pub const HISTORY: Self = Self(1 << 5);
    /// a field something is typed into, which is a modal rather than a panel
    pub const TYPING: Self = Self(1 << 6);
    /// the open file's todos are `TODO:` comments: a key that needs a document is
    /// not offered at all, because there is nothing there for it to act on (RFC-015)
    pub const COMMENT: Self = Self(1 << 7);

    /// Both places, for a key a collection shares with the panels that edit a file.
    pub const fn or(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    fn works(self, here: Self) -> bool {
        self == Self::ANY || self.0 & here.0 != 0
    }
}

/// Where a binding goes in the status bar: the row that says where you can go, the
/// row that says what you can do here, or nowhere — the page only.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Bar {
    No,
    Nav,
    Verb,
}

/// One binding: the keys as a terminal writes them, what they do, and what they need.
pub struct Binding {
    pub keys: &'static str,
    pub does: &'static str,
    pub panes: Where,
    pub bar: Bar,
    /// Nothing happens unless something is under the cursor: a todo to check, a file
    /// to delete, an entry to undo.
    pub something: bool,
    /// Places the key does not work in, even where its panels do: checking a todo
    /// comment is not an operation, so `x` is not offered in a comment todo file.
    pub without: Where,
}

impl Binding {
    const fn new(keys: &'static str, does: &'static str, panes: Where, bar: Bar) -> Self {
        Self {
            keys,
            does,
            panes,
            bar,
            something: false,
            without: Where::ANY,
        }
    }

    const fn on_something(self) -> Self {
        Self {
            something: true,
            ..self
        }
    }

    /// A key that exists in one kind of file and not the other: `x` and `n` and the
    /// move keys need a document to write in.
    const fn but_not(self, without: Where) -> Self {
        Self { without, ..self }
    }

    fn works(&self, here: Where, something: bool) -> bool {
        // a plain bit test, unlike `panes`: "not here" means none of its bits are set,
        // and an empty mask excludes nothing
        self.panes.works(here) && (!self.something || something) && self.without.0 & here.0 == 0
    }
}


/// Every key the screen answers to, in the order they are read.
///
/// One row per *spelling*, not per panel: a key that works in three panels is one row
/// with three bits, and both the page and the bar work out for themselves where to
/// show it. A row that is not here is a key nobody can find.
pub const BINDINGS: &[Binding] = &[
    // ---- getting around: the same in every panel, because none of it is about the
    // thing under the cursor
    Binding::new("tab", "next section", Where::ANY, Bar::Nav),
    Binding::new("shift+tab", "prev", Where::ANY, Bar::Nav),
    // the number in a panel's title is the key that jumps to it, and `0` is the
    // panel a person works in most — the same 0 lazygit uses
    Binding::new("0-3", "that panel", Where::ANY, Bar::No),
    Binding::new("→", "into view", Where::ANY, Bar::Nav),
    Binding::new("←", "back out", Where::ANY, Bar::Nav),
    Binding::new("g / G", "first / last", Where::ANY, Bar::No),
    Binding::new("home/end", "first / last", Where::ANY, Bar::No),
    // the same jump again: a terminal that drops the modifier off an arrow key sends
    // only the letter, and the letter is what the bar offers
    Binding::new("shift+↑/↓", "first / last", Where::ANY, Bar::No),
    Binding::new(text::SEARCH_KEY, "search", Where::ANY, Bar::Nav),
    Binding::new("?", "keys", Where::ANY, Bar::Nav),
    Binding::new("q", "quit", Where::ANY, Bar::Nav),
    Binding::new("ctrl+c", "quit anywhere", Where::ANY, Bar::No),
    Binding::new("t", "line height", Where::ANY, Bar::No),
    Binding::new("esc", "dismiss it", Where::ANY, Bar::No),
    // the sidebar is the only place with a next section: in the todo panels this
    // modifier moves the todo, which is what the row under the todos says. Listed
    // under the two sections that are not the file panel, which already has `tab`
    // and `shift+tab` on the row above and whose own key column has no room for an
    // eight-cell key
    Binding::new(
        "opt+↑/↓",
        "prev / next",
        Where::OUTLINE.or(Where::HISTORY),
        Bar::No,
    ),
    // ---- the file panel
    // "nav" in every panel, never the thing under the cursor: the arrows move the
    // cursor, and naming what it lands on read as an action on it — `↑/↓ todo` sat
    // beside `x check` and `d delete` like a third verb. One idea, one word.
    Binding::new("↑/↓", "nav", Where::FILES, Bar::Verb),
    Binding::new("n", "new file", Where::FILE_ROW, Bar::Verb),
    Binding::new("d", "delete file", Where::FILE_ROW, Bar::Verb),
    Binding::new("h", "hide / unhide", Where::FILES, Bar::Verb),
    Binding::new("v", "what to list", Where::FILES, Bar::No),
    // ---- the index of headings
    Binding::new("↑/↓", "nav", Where::OUTLINE, Bar::Verb),
    Binding::new("n", "new heading", Where::OUTLINE, Bar::Verb),
    // ---- a collection: the one key that is its own, before the rows it shares with
    // the panels below
    Binding::new("enter", "open file", Where::COLLECTION, Bar::Verb).on_something(),
    // ---- the panels that edit the open file's todos, and a collection's rows
    Binding::new("↑/↓", "nav", Where::TODO.or(Where::COLLECTION), Bar::Verb),
    Binding::new("x", "check", Where::TODO.or(Where::COLLECTION), Bar::Verb)
        .on_something()
        .but_not(Where::COMMENT),
    // the second spelling of `x`: a terminal always sends the letter, so the bar keeps
    // the shorter row and the page is where one key is told twice
    Binding::new("space", "check", Where::TODO.or(Where::COLLECTION), Bar::No)
        .on_something()
        .but_not(Where::COMMENT),
    Binding::new("K / J", "move", Where::TODO, Bar::Verb)
        .on_something()
        .but_not(Where::COMMENT),
    Binding::new("ctrl+↑/↓", "move", Where::TODO, Bar::No)
        .on_something()
        .but_not(Where::COMMENT),
    Binding::new("d", "delete", Where::TODO.or(Where::COLLECTION), Bar::Verb).on_something(),
    Binding::new("n", "add", Where::TODO, Bar::Verb).but_not(Where::COMMENT),
    Binding::new("e", "edit", Where::TODO.or(Where::COLLECTION), Bar::Verb).on_something(),
    Binding::new("A-D", "prio", Where::TODO.or(Where::COLLECTION), Bar::Verb).on_something(),
    Binding::new("0", "clear prio", Where::TODO.or(Where::COLLECTION), Bar::No)
        .on_something(),
    // ---- the history
    Binding::new("↑/↓", "nav", Where::HISTORY, Bar::Verb),
    Binding::new("x", "undo / redo", Where::HISTORY, Bar::Verb).on_something(),
    Binding::new("space", "undo / redo", Where::HISTORY, Bar::No).on_something(),
    Binding::new("d", "forget", Where::HISTORY, Bar::Verb).on_something(),
    // ---- every panel with something to copy: a file row the path, a todo the line, a
    // heading its text, a collection its name
    Binding::new(
        "c",
        "copy",
        Where::FILES
            .or(Where::OUTLINE)
            .or(Where::TODO)
            .or(Where::COLLECTION),
        Bar::Verb,
    ),
    // ---- a field: the two keys that commit or leave, and the one that is not obvious
    Binding::new("esc", "cancel", Where::TYPING, Bar::No),
    Binding::new("enter", "submit", Where::TYPING, Bar::No),
    Binding::new("ctrl+u", "empty the search", Where::TYPING, Bar::No),
];

/// The bindings that work in any of these places, in table order.
pub fn bindings(panes: &[Where]) -> Vec<&'static Binding> {
    BINDINGS
        .iter()
        .filter(|binding| panes.iter().any(|here| binding.panes.works(*here)))
        .collect()
}

/// What a panel offers and no other panel does: `enter` is a collection's own key.
pub fn alone(panes: Where) -> Vec<&'static Binding> {
    BINDINGS
        .iter()
        .filter(|binding| binding.panes == panes)
        .collect()
}

/// What is true of the app right now, in the same bits as `Where`.
fn here(app: &App) -> Where {
    let mut here = match app.pane {
        Pane::Files => Where::FILES,
        Pane::Outline => Where::OUTLINE,
        Pane::View | Pane::Todos => Where::TODO,
        Pane::Collection => Where::COLLECTION,
        Pane::History => Where::HISTORY,
    };
    if app.pane == Pane::Files && app.selected_file().is_some() {
        here = here.or(Where::FILE_ROW);
    }
    // the kind of file decides what can be done to its todos, and it is the open
    // file that is being edited — a collection's rows open their own files
    if matches!(app.pane, Pane::View | Pane::Todos | Pane::Collection) && app.comment() {
        here = here.or(Where::COMMENT);
    }
    here
}

/// Is there anything under the cursor for a key to act on?
fn something(app: &App) -> bool {
    match app.pane {
        Pane::Files => app.selected_row().is_some(),
        Pane::Outline => app.outline_idx < app.headings().len(),
        Pane::View | Pane::Todos => app.current_task().is_some(),
        Pane::Collection => app.task_idx < app.items.len(),
        Pane::History => app.history_idx < app.file_history().len(),
    }
}

/// The keys every panel shares. Where you are is not said here: the active panel
/// carries a dot, and the numbers belong to the file they count. Never a message
/// either — what went wrong is drawn under the thing it went wrong on.
///
/// The sidebar keys name their destination — `tab index`, not `tab section` —
/// because which panel `tab` reaches is the thing a person cannot guess.
pub fn general_keys(app: &App) -> String {
    BINDINGS
        .iter()
        .filter(|binding| binding.bar == Bar::Nav && binding.works(here(app), something(app)))
        .filter_map(|binding| nav(app, binding))
        .collect::<Vec<_>>()
        .join(" · ")
}

/// A row of the bar's first line: the ones that name a destination say which, and the
/// sideways arrow that has nowhere to go says nothing at all.
fn nav(app: &App, binding: &Binding) -> Option<String> {
    let named = match binding.keys {
        "tab" => format!("tab {}", app.focus_target(true)?.name()),
        "shift+tab" => format!("shift+tab {}", app.focus_target(false)?.name()),
        "←" if app.pane == Pane::View || app.pane == Pane::Collection => {
            format!("← {}", app.last_sidebar.name())
        }
        // what the arrow would put you in — a file, or the collection the cursor is
        // on — rather than the word "view"
        "→" if app.pane != Pane::View && app.pane != Pane::Collection && !app.single => {
            let what = match app.selected_row().and_then(Row::collection) {
                Some(collection) => collection.to_string(),
                None => shorten(&app.display(&app.current().path), SHORT_PATH),
            };
            format!("→ {what}")
        }
        "←" | "→" => return None,
        _ => format!("{} {}", binding.keys, binding.does),
    };
    Some(named)
}

/// The bar's second line: what will actually happen in the panel that has the
/// keyboard. A key whose thing is not under the cursor is not offered — a key that
/// would do nothing is worse than a shorter line.
pub fn section_keys(app: &App) -> Vec<String> {
    BINDINGS
        .iter()
        .filter(|binding| binding.bar == Bar::Verb && binding.works(here(app), something(app)))
        .map(|binding| format!("{} {}", binding.keys, does_now(app, binding)))
        .collect()
}

/// `h` is a switch, so its row says which way it goes.
fn does_now(app: &App, binding: &Binding) -> &'static str {
    if binding.keys != "h" {
        return binding.does;
    }
    if app.show_all {
        text::HIDE_MD
    } else {
        text::UNHIDE_MD
    }
}

/// A key as the terminal described it: `Down + ALT`, `x`, `shift+tab`.
pub fn key_name(key: &ratatui::crossterm::event::KeyEvent) -> String {
    use ratatui::crossterm::event::{KeyCode, KeyModifiers};

    let code = match key.code {
        KeyCode::Char(character) => character.to_string(),
        KeyCode::Up => "Up".to_owned(),
        KeyCode::Down => "Down".to_owned(),
        other => format!("{other:?}"),
    };
    let modifiers = [
        (KeyModifiers::SHIFT, "SHIFT"),
        (KeyModifiers::CONTROL, "CTRL"),
        (KeyModifiers::ALT, "ALT"),
        (KeyModifiers::SUPER, "SUPER"),
        (KeyModifiers::HYPER, "HYPER"),
        (KeyModifiers::META, "META"),
    ];
    let held: Vec<&str> = modifiers
        .iter()
        .filter(|(flag, _)| key.modifiers.contains(*flag))
        .map(|(_, name)| *name)
        .collect();
    if held.is_empty() {
        code
    } else {
        format!("{code} + {}", held.join(" + "))
    }
}

#[cfg(test)]
mod tests {
    use crate::state::action::Pane;

    #[test]
    fn the_hint_for_h_says_which_way_the_switch_goes() {
        let (_tmp, mut app) = crate::testing::app(&[("TODOs.md", "- [ ] one\n")]);
        app.pane = Pane::Files;
        // nothing hidden yet, so pressing it hides nothing: it *unhides* the rest
        assert!(super::section_keys(&app).join(" · ").contains("h unhide *.md"));

        app.show_all = true;
        assert!(super::section_keys(&app).join(" · ").contains("h hide *.md"));

        // the collection panel is for reading todos: the scan's switch belongs to
        // the panel that lists the files
        app.pane = Pane::Collection;
        assert!(!super::section_keys(&app).join(" · ").contains("hide"));

        // and in a panel about one document there is no hint at all
        app.pane = Pane::View;
        let elsewhere = super::section_keys(&app).join(" · ");
        assert!(!elsewhere.contains("hide"), "{elsewhere}");
    }

    #[test]
    fn a_collection_row_offers_no_new_file_and_no_delete() {
        let (_tmp, mut app) = crate::testing::app(&[("TODOS.md", "- [ ] one #home\n")]);
        app.pane = Pane::Files;
        app.row_idx = 0;
        let on_a_file = super::section_keys(&app).join(" · ");
        assert!(on_a_file.contains("new file"), "{on_a_file}");
        assert!(on_a_file.contains("delete file"), "{on_a_file}");

        // the same panel, one row down: the collections. `n` and `d` are gone
        // because they are about a file; `h` stays, because it is about the scan
        app.row_idx = app.files.len();
        let on_a_collection = super::section_keys(&app).join(" · ");
        assert!(!on_a_collection.contains("new file"), "{on_a_collection}");
        assert!(!on_a_collection.contains("delete"), "{on_a_collection}");
        assert!(on_a_collection.contains("unhide *.md"), "{on_a_collection}");
    }

    #[test]
    fn the_navigation_names_where_it_goes() {
        let (_tmp, mut app) =
            crate::testing::app(&[("TODOs.md", "# one\n\n- [ ] a\n\n## two\n\n- [ ] b\n")]);
        let line = super::general_keys(&app);
        assert!(line.contains("tab index"), "{line}");
        assert!(line.contains("shift+tab history"), "{line}");
        assert!(line.contains("→ TODOs.md"), "{line}");

        app.pane = Pane::Outline;
        let line = super::general_keys(&app);
        assert!(line.contains("tab history"), "{line}");
        assert!(line.contains("shift+tab files"), "{line}");

        app.pane = Pane::View;
        let line = super::general_keys(&app);
        assert!(line.contains("← files"), "{line}");
        assert!(!line.contains("→ "), "{line}");
        assert!(
            !line.contains("tab"),
            "the view is not in the sidebar, so tab does nothing here: {line}"
        );
    }

    #[test]
    fn the_arrow_hint_with_a_deep_path_reads_as_the_file() {
        let (_tmp, mut app) = crate::testing::app(&[
            ("a/deeply/nested/place/where/TODOs.md", "- [ ] a\n"),
            ("b/TASKS.md", "- [ ] b\n"),
        ]);
        app.pane = Pane::Files;
        let line = super::general_keys(&app);
        assert!(line.contains("→ …"), "{line}");
        assert!(line.contains("TODOs.md"), "{line}");
        assert!(!line.contains("a/deeply/nested"), "{line}");
    }

    #[test]
    fn the_sideways_arrows_name_the_panel_they_reach() {
        let (_tmp, mut app) =
            crate::testing::app(&[("a/TODOs.md", "- [ ] a\n"), ("b/TASKS.md", "- [ ] b\n")]);
        let line = super::general_keys(&app);
        assert!(line.contains("→ a/TODOs.md"), "{line}");

        // entering from the index comes back to the index
        app.pane = Pane::Outline;
        let _ = app.enter_view();
        let line = super::general_keys(&app);
        assert!(line.contains("← index"), "{line}");
    }

    #[test]
    fn navigation_and_quit_are_not_repeated_per_section() {
        let (_tmp, mut app) = crate::testing::app(&[("TODOs.md", "- [ ] a\n")]);
        for pane in [
            Pane::Files,
            Pane::Outline,
            Pane::View,
            Pane::Todos,
            Pane::History,
        ] {
            app.pane = pane;
            let hints = super::section_keys(&app);
            assert!(
                hints
                    .iter()
                    .all(|hint| !hint.contains("tab") && !hint.contains("quit")),
                "{hints:?}"
            );
        }
    }

    #[test]
    fn the_history_section_offers_undo_and_not_editing() {
        let (_tmp, mut app) = crate::testing::app(&[("TODOs.md", "- [ ] a\n")]);
        app.pane = Pane::History;
        // nothing has been written, so there is no entry to undo and the bar offers
        // only the cursor
        assert_eq!(super::section_keys(&app), ["↑/↓ nav"]);

        // one check, and there is something to undo
        app.pane = Pane::Todos;
        let _ = app.press(ratatui::crossterm::event::KeyCode::Char('x'));
        app.pane = Pane::History;
        let hints = super::section_keys(&app);
        assert!(hints.contains(&"x undo / redo".to_owned()), "{hints:?}");
        assert!(hints.contains(&"d forget".to_owned()), "{hints:?}");
        for absent in ["n add", "e edit", "A-D prio"] {
            assert!(!hints.contains(&absent.to_owned()), "{absent} in {hints:?}");
        }
    }

    #[test]
    fn a_key_with_nothing_under_the_cursor_is_not_offered() {
        // a file that no longer holds a todo: the panels that edit todos have nothing
        // to edit, so the bar says only what still works
        let (_tmp, mut app) = crate::testing::app(&[("TODOs.md", "# empty\n\n- [ ] gone\n")]);
        app.pane = Pane::View;
        let _ = app.press(ratatui::crossterm::event::KeyCode::Char('d'));
        let _ = app.press(ratatui::crossterm::event::KeyCode::Char('Y'));
        let hints = super::section_keys(&app);
        assert_eq!(hints, ["↑/↓ nav", "n add", "c copy"], "{hints:?}");

        // a collection's `enter` is about the row the cursor is on, and there is none
        app.pane = Pane::Collection;
        assert!(super::section_keys(&app).iter().all(|hint| !hint.contains("enter")));
    }
}
