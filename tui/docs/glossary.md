# The words of the terminal client

Every name a person reads on screen, hears in a conversation about this program, or
finds in an RFC. One meaning each, and one place they are written down.

The code follows this list. When a name here and a name in the code disagree, the
list is right and the code is the bug.

## The screen

| word | what it is |
| --- | --- |
| **header** | the top line: `✅ aricma-lazytodo · /where/it/was/opened`, and the right-hand end of it, where a **success message** appears |
| **status bar** | the bottom line: the keys that work where the cursor is, and a **failure** or a rule explained |
| **panel** | one framed box in the body of the screen. A panel draws one thing and has one title |
| **thumb** | the dim `▐` on a panel's inner right edge, when its rows do not fit. No track, and none at all when they do (RFC-001) |
| **focused panel** | the panel the cursor is in. Its border is bright, its title carries `●`, and the status bar speaks for it |
| **band** | the highlight on the row the cursor is on. A banded row keeps its tag and assignee colours |
| **gutter** | the dim column of line numbers down the left of a panel that shows document lines |
| **popover** | a floating box over the panels: search, the keys page, a question, or a field. One at a time, and it owns the keyboard while it is up |
| **roomy** | the setting that leaves a blank row between rows, so a list reads as paragraphs |

## The panels

This is the part that used to drift: several panels show todos, and calling all of
them "the todos panel" made every sentence about the screen ambiguous.

| panel | title on screen | what it draws |
| --- | --- | --- |
| **files panel** | `files` | one row per file that holds todos, then one row per collection. It is the list of places, not of todos |
| **index panel** | `index` | the headings of the open file, with the todos under them folded away. Where a heading is made |
| **history panel** | `history` | what this session did, newest first, and the undo cursor — see **history** below |
| **todos panel** | `todos`, or `<tag> todos` / `@<person> todos` | the todos of exactly one thing: a todos file in a single-file session, or a collection. Numbered, one row per todo, wrapping under its own body |
| **file view** | the file's path | the whole document: prose, headings and todos, every line numbered. It is not a todos panel — it shows a file |

**files panel, index panel, history panel, todos panel, file view.** Never "the
left panel", "the right side", "the middle" — the layout moves, the names do not.

## The document

| word | what it is |
| --- | --- |
| **todo line** | one line of markdown: a bullet, a checkbox, an optional priority, a body |
| **lead** | everything in front of the body: indent, bullet, checkbox, space — or, for a **comment todo**, the indent and the comment up to `TODO: `. Drawn dim, never editable here |
| **body** | the text after the lead: the part a person writes |
| **marker** | `- [ ]` or `- [x]` — the bullet and the state |
| **priority** | `(A)`…`(D)`, straight after the checkbox — or, for a **comment todo**, straight after `TODO: `. Yellow, and the only positional rule in the grammar |
| **tag** | `#home` — cyan. A tag is a **collection**, and a tag is where todos live |
| **assignee** | `@sam` — magenta. A person is a **collection** too |
| **prose** | every line of a file that is not a todo — a bare bullet included, and in a **comment todo file** every line of code. Drawn dim, one row, clipped |
| **finding** | a line that meant to be a todo and is not one (`--check`). Reported, never silently rewritten (RFC-013) |
| **heading** | a markdown `#` line, what the index panel is made of |
| **chapter** | a heading and everything under it, up to the next heading of any level: its **introduction**, then its **cluster** |
| **introduction** | the text of a chapter before its first todo: paragraphs, tables, images, code |
| **cluster** | the todos of a chapter, together at the end of it, in written order. No key splits one (RFC-012) |

## Places

| word | what it is |
| --- | --- |
| **collection** | a tag or an assignee: the todos across every file that carry it, grouped by file. It has no line numbers of its own, because its lines are other files' lines |
| **candidate file** | in the named scan, a file the walk reads: `TODO*`, `TASK*` or `README.md` ending in `.md`, or any other text file, which is read for `TODO:` comments (RFC-016) |
| **listed file** | a candidate file that is worth a row: it holds a todo, a line that meant to be one, or nothing but headings |
| **root** | the folder (or the one file) the session was opened on. The header shows it resolved |
| **single-file session** | a session opened on one file. No files panel, no file view — the todos of that file are the right-hand column |
| **scan** | the walk that finds listed files: git's own file list when there is a work tree, the folder list otherwise, up to ten levels deep and never into what git ignores (RFC-017) |
| **dependency** | a todo that names another todo. Drawn as a chain in the history panel's graph, not in the todos panel |
| **todo document** | a `.md` file whose `- [ ]` lines are its todos. The original source, and the one the keys were written for |
| **comment todo file** | any other text file the tree holds, whose `TODO:` comments are its todos. A code file with one note in it is a comment todo file (RFC-015) |
| **comment todo** | one `TODO:` comment, one line: `    // TODO: (A) fix the retry #infra`. The word is in capitals, a comment marker stands in front of it with no quote between, or a word in a sentence would be a todo. Everything in front of the text is the line's, and the tool never rewrites it (RFC-016). Finding one is a guess that leans narrow — what it misses is RFC-019's table |
| **source** | which of the three a todo came from: a **todo document**, a **comment todo file**, or a **collection**. The source decides the keys (RFC-015) |
| **fold** | in a comment todo file, the `…` row that stands for the code between the window around a todo — a line above it, three below — and the next window (RFC-018) |

## Doing

| word | what it is |
| --- | --- |
| **edit** | changing a todo's body where it sits (`e`), the row becoming the field |
| **add** | a new todo: the empty row goes in first and the cursor moves onto it, then the field floats over the screen for the words. Enter fills the row in, `esc` takes it back out |
| **check / uncheck** | the one key that changes a todo's state, and a **history entry** |
| **move** | changing which todo a line sits after. The file's order is the only order, and across a heading the todo joins the far chapter's **cluster** |
| **history** | the session's own list of edits that can be undone, with the file's real history read from it — a **history entry** names what it did (`new todo`, `set prio B`) and carries a dot, `●` in effect or `○` undone, never a checkbox (RFC-008) |
| **copy** | putting the selected thing on the clipboard: a path or a heading as plain text, a **todo** as an OPS document (RFC-014) |
| **paste** | a string arriving from the clipboard. Text in a field; an OPS document anywhere todos live, written out as todo lines — the file never receives JSON |
| **refusal** | a reason a key cannot act, said in words where the thing it blocks is. Not a failure |
| **failure** | something went wrong that the program did not choose: a write that did not land, a file that moved |

## Where the words live

- `lazytodo.md` — how the screen behaves, as prose.
- `docs/rfc/` — one RFC per behaviour, the current answer and why. Marketing reads
  this directory; nothing here is written for it.
- `text.rs` — every word a person reads on screen. If it is not there, it is not a
  word of this program.
