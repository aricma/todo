//! Every word the user reads: the names of things, the controls on a border, and
//! the sentences that say why something did not happen.
//!
//! One file because a product has one voice. If two places draw the same idea,
//! they say it the same way, and fixing a sentence is one edit.

// --- what things are called -------------------------------------------------

/// The name the window carries, and the wave that it is a list you can finish.
///
/// `aricma-todo` is the project; this is its screen, `aricma-lazytodo`.
pub const APP: &str = "✅ aricma-lazytodo";

pub const FILES: &str = "files";
pub const INDEX: &str = "index";
pub const HISTORY: &str = "history";
pub const TODOS: &str = "todos";
pub const COLLECTION: &str = "collection";
pub const VIEW: &str = "view";
pub const GENERAL: &str = "general";
/// The keys a field takes: a modal rather than a panel, but it is still a place the
/// keyboard can be, and the page has to say what works there.
pub const TYPING: &str = "typing";
/// Under the collection's own key: the rest are the todo panel's rows, save two. Two
/// lines, because one row of forty cells is wider than a column of keys.
pub const COLLECTION_KEYS_NOTE: [&str; 2] = ["the todo keys above,", "but no n and no K / J"];

// --- what a history entry calls what it did ---------------------------------
//
// One shape for all of them — a verb and its object, in the same three or four
// words — because a column of entries is read down, not across, and "add" beside
// "priority B" makes the eye do the work. `set prio B` not `priority B`: the letter
// is a value, and what happened to it is `set`.

// `new todo` and `delete todo` are the popups' titles too: one word for one thing
pub const EDIT_TODO: &str = "edit todo";
pub const MOVE_TODO: &str = "move todo";
pub const CHECK_TODO: &str = "check todo";
pub const UNCHECK_TODO: &str = "uncheck todo";
pub const CLEAR_PRIO: &str = "clear prio";

/// The priority a todo was given: `set prio B`.
pub fn set_prio(letter: char) -> String {
    format!("set prio {letter}")
}

// --- the one switch that says which way it goes -----------------------------
//
// `h` is a toggle, so the bar names what pressing it *now* would do rather than
// what it did: a key that says "hide" while everything is visible is a key that
// reads as its opposite.

pub const HIDE_MD: &str = "hide *.md";
pub const UNHIDE_MD: &str = "unhide *.md";

// --- popup titles -----------------------------------------------------------

pub const SEARCH: &str = "search";

/// How many todos a query found: `12 matches`, `1 match`.
pub fn matches(count: usize) -> String {
    match count {
        1 => "1 match".to_owned(),
        count => format!("{count} matches"),
    }
}
pub const NEW_TODO: &str = "new todo";
pub const NEW_HEADING: &str = "new heading";
pub const CONFLICT: &str = "changed on disk";
pub const KEEP_MINE: &str = "Y keep mine";
pub const TAKE_THEIRS: &str = "t take theirs";
pub const CONFLICT_KEYS: &str = " Y keep mine · t take theirs · q quit ";
pub const NEW_FILE: &str = "new todos file";
pub const KEYS: &str = "keys";
pub const DELETE_TODO: &str = "delete todo";
pub const DELETE_FILE: &str = "delete file";

// --- controls, right-aligned in a popup's bottom border ---------------------
//
// Padded, because a border title is drawn between two corners and needs a column
// either side of it.

pub const CANCEL_ADD: &str = " esc cancel · enter add ";
pub const CANCEL_CREATE: &str = " esc cancel · enter create ";
pub const CANCEL_DELETE_TODO: &str = " esc cancel · Y delete ";
pub const CANCEL_DELETE_FILE: &str = " esc cancel · Y delete ";
pub const CLOSE_KEYS: &str = " esc close ";
pub const LAST_KEY: &str = " press a key to see what arrives ";

// --- the same controls, as the bar spells them ------------------------------
//
// Unpadded and separate: the bar joins its keys with ` · ` itself, so a padded
// string there would be a double space.

pub const ESC_CANCEL: &str = "esc cancel";
pub const ENTER_ADD: &str = "enter add";
pub const ENTER_SAVE: &str = "enter save";
pub const ENTER_CREATE: &str = "enter create";
pub const ENTER_JUMP: &str = "enter jump";
pub const ESC_CLOSE: &str = "esc close";
pub const Q_QUIT: &str = "q quit";
pub const Y_DELETE: &str = "Y delete";
pub const PICK: &str = "↑/↓ pick";
pub const SEARCH_KEY: &str = "f";
pub const CLEAR_SEARCH: &str = "ctrl+u clear";

/// The keys in a search's own border: leaving is always possible, going somewhere
/// is not, so `enter` is only offered while a hit is selected.
pub fn search_keys(can_jump: bool) -> String {
    if can_jump {
        format!(" {ESC_CANCEL} · {ENTER_JUMP} ")
    } else {
        format!(" {ESC_CANCEL} ")
    }
}

// --- what went wrong, or would ---------------------------------------------

/// The heading a collection shows above its todos. Rendered, never written: a
/// collection is a view of files, not a file.
pub fn collection_heading(kind: &str, name: &str) -> String {
    format!("# {kind} {name} todo collection")
}

/// What the history panel is showing while a collection is open.
pub fn history_of(path: &str) -> String {
    format!("{HISTORY} · {path}")
}

/// A collection is a view, not a place: there is no line to move a todo to.
pub const NO_MOVING: &str = "a collection has no order of its own to move in";

/// A conflict, resolved by taking the file's version.
pub fn took_theirs() -> String {
    "took the file's version".to_owned()
}

/// A conflict, resolved by keeping ours.
pub fn kept_mine() -> String {
    "kept your version".to_owned()
}

/// The heading a new file is born with: the folder it lives in.
pub fn new_file(folder: &str) -> String {
    if folder.is_empty() {
        String::new()
    } else {
        format!("# {folder} todos\n\n")
    }
}

/// A heading with nothing in it is not a heading.
pub const NEEDS_A_HEADING: &str = "a heading needs a title";

/// A todo with nothing in it is not a todo.
pub const NEEDS_A_TITLE: &str = "a todo needs a title";

/// A file's path, or a todo's line, is on the clipboard.
pub fn copied(what: &str) -> String {
    format!("copied {what}")
}

/// A search with no result is not an error, so it is not coloured like one.
pub const NO_MATCH: &str = "no todo matches";

/// The line a write anchored to no longer reads as it left it.
pub const LINE_MOVED: &str = "that line changed since then";

/// Appended to `LINE_MOVED`: the app re-reads the file on its own.
pub const IT_RELOADS: &str = "it reloads on its own";

/// A write cannot be redone before the write it undoes.
pub const BLOCKED: &str = "enable the write below it first";

/// There is nothing above or below to swap with.
pub fn nothing_to_swap(way: &str) -> String {
    format!("nothing {way} it to swap with")
}

/// A new file never overwrites one.
pub fn already_there(name: &str) -> String {
    format!("{name} is already there — nothing is overwritten")
}

/// A path that walks out of the tree the tool was opened on.
pub fn keep_inside(root: &str) -> String {
    format!("keep the path inside {root}")
}

/// A file over the line limit is listed, never opened.
pub fn too_long(shown: &str, lines: usize) -> String {
    format!("{shown}: more than {lines} lines, not opened")
}
