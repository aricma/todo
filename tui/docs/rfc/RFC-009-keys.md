# RFC-009: Keys, focus and crossing

- **Date:** 2026-10-02
- **Related:** [RFC-001](RFC-001-panels-and-their-names.md), [RFC-003](RFC-003-editing-in-place.md)

## Context

A terminal program has no menus and no mouse worth the name, so the keyboard *is* the
interface: every capability of this program is one keystroke away, and a keystroke a
person cannot find is a capability that does not exist. The keymap is therefore data,
not code — `tui/src/ui/hints.rs` holds one row per spelling, with the places it works
and whether it belongs in the status bar — and three surfaces are generated from it:
the status bar's two rows, and the keys page (`?`).

The hard part is not the letters. It is that a modified key does not reach a program
intact on every terminal: macOS Terminal.app with "use option as meta" sends
`ESC ESC [ A` for `opt+↑`, which a program cannot distinguish from `esc` followed by
`[` and `A`. Every binding that needs a modifier therefore has a spelling that needs
none, and the bar offers the spelling that always arrives.

## Behaviour

1. One key **MUST** mean one idea, and `n` is the model: a new todo where todos are
   edited, a new file where files are chosen, a new heading in the index.
2. Navigation **MUST** be the same in every panel — `tab` / `shift+tab` between
   sections, `→` into the view, `←` back out, `g`/`G`, `home`/`end`, `f`, `?`, `q`,
   `ctrl+c` anywhere — because none of it is about the thing under the cursor.
3. A panel **MUST** own its verbs: `n`, `d`, `x`, `e`, `A`–`D`, `0`, `K`/`J`, `c`,
   `v`, `h` act only where they make sense, and the status bar **MUST** offer only the
   ones that work here *and* have something under the cursor. A key that would do
   nothing is worse than a shorter line.
4. `x` **MUST** stay the check key in the todos panel and the undo key in the history
   panel — the same gesture ("this row, now") for the panel's own verb — and `space`
   **MUST** be its second spelling, because a terminal always sends the letter and a
   hand reaches for the space bar.
5. Every modified binding **MUST** have an unmodified spelling: `K`/`J` beside
   `ctrl+↑/↓`, `home`/`end` beside `shift+↑/↓`, `space` beside `x`.
6. `d` **MUST** ask before a delete, and only a capital `Y` **MUST** confirm: the
   question is a one-key accident away from the answer, and `n`/`enter`/anything else
   cancels.
7. A conflict **MUST NOT** be dismissible: `keep mine` and `take theirs` are the only
   two keys, because there is no third state in which nothing is chosen.
8. A field (a todo, a heading, the search query, the new-file path) **MUST** answer
   `esc` cancel and `enter` submit, and nothing else may be a modal key while it is up.
   A refusal **MUST** block `enter` rather than fail after it.
9. `q` **MUST** quit from any panel, `esc` **MUST** cancel or dismiss, `ctrl+c` **MUST**
   quit from anywhere including inside a field, and `t` **MUST** toggle the blank row
   between rows.
10. The keys page (`?`) **MUST** list every binding in `hints.rs`, including the
    spellings the bar leaves out, and a binding that is not in `hints.rs` **MUST NOT**
    exist.

## The map, and why each key is what it is

| where | key | does | why this key |
| --- | --- | --- | --- |
| anywhere | `tab` / `shift+tab` | next / prev section | the only key every terminal agrees on; the bar names the destination (`tab index`) because which section comes next is the thing a person cannot guess |
| anywhere | `0`–`3` | that panel | the number is in the panel's title (`[3] history`), so the key is the label: a person reading the screen can already press it. `0` is the todos panel, as it is in lazygit, because that is the panel a person is in most. A panel that is not on screen takes no number — `1` in a single-file session does nothing |
| anywhere | `→` / `←` | into the right-hand panel / back out | the two arrows that look like the two columns; the bar names what `→` would open — a file path, a `#home`, or `todos` in a single-file session, where the right-hand panel is the todos of the one file |
| anywhere | `g` / `G`, `home`/`end`, `shift+↑/↓` | first / last | three spellings of one jump, for the three kinds of terminal (letters, always-sent keys, modifier-reporting keys) |
| anywhere | `f` | search | `cmd+f` and `ctrl+f` belong to the terminal or to scrolling; one letter is free and memorable |
| anywhere | `?` | keys page | the one key everyone tries |
| anywhere | `q` / `ctrl+c` | quit | `q` for the panel-level quit, `ctrl+c` for anywhere, including inside a field where `q` is a letter |
| anywhere | `t` | line height | "tall" rows; a display setting, not a document one, so it must not shadow a letter a person types |
| every panel | `↑/↓` | nav | the arrows move the *cursor*, not the thing under it. The bar used to name that thing (`↑/↓ todo`, `↑/↓ entry`, `↑/↓ file / collection`, `↑/↓ heading`), which read as a series of verbs beside `x check` and `d delete`; one idea is one word, so every panel says the same |
| files panel | `n` | new file | new where files are chosen |
| files panel | `d` | delete file | the panel's own destructive verb, behind `Y` |
| files panel | `h` | hide / unhide | "hidden files": widen the scan to every `*.md` and back |
| files panel | `v` | what to list | cycles files → tags → people, the three kinds of row |
| index panel | `n` | new heading | new where headings are |
| todos panel, file view | `x` / `space` | check | the checkbox, and the key that cannot arrive wrong |
| todos panel, file view | `K` / `J`, `ctrl+↑/↓` | move the todo | the letters sit beside the arrows, so a terminal that eats the modifier still moves |
| todos panel, file view | `e` | edit in place | RFC-003 |
| todos panel, file view | `A`–`D` | set / clear priority | the letters are the priorities, and the letter a todo already carries clears it: `0` used to do that too, and gave up its key when `0` became the todos panel's number, which cost nothing — two spellings of "no priority" were one too many |
| todos panel, file view, collection | `d` | delete | the same verb as the file panel's, on the row's own kind |
| collection | `enter` | open file | the one key a collection has that no other panel does: it says "show me where this line lives" |
| history panel | `x` / `space` | undo / redo | the panel's own verb: "this row, now" |
| history panel | `d` | forget | forget is a different act from undo, and it is destructive, so it is a letter, not `x` |
| files, index, todos, collection | `c` | copy | "copy", and it never writes anything (RFC-004) |

## Why not the alternative

**A command palette** (one key, then a search over verbs) was rejected as the primary
interface: this program is used with one hand and one eye on the file, and a palette
costs two keystrokes and a read before anything happens. It also cannot show what works
*here*, which is what the status bar is for.

**`vim`-shaped letter keys everywhere** (`dd`, `yy`, counts) was rejected: the program is
used by people who do not live in `vim`, and a modal grammar on top of a modal grammar
is a thing to learn, not a thing to use.

**Modified keys only** (no unmodified spellings) was rejected empirically: on the
author's own terminal `opt+↑` does not arrive, and a feature that only works on a
terminal the user does not have is not a feature.

## Consequences

- Adding a binding means adding a row to `hints.rs`; the page and the bar follow, and a
  test asserting the page lists every row keeps them honest.
- Two spellings per idea doubles the paths through the state machine, which is why the
  second spellings are handled in one place (`x | space`) and never in the panels.
- `d` in the history panel means something different from `d` in the todos panel. That
  is allowed by rule 3 (the panel owns its verbs) and is the one collision in the map;
  the bar always spells out which is which.

## Open questions

- Should the second spellings be hidden from the keys page, or is a full list more
  honest than a short one?
- `v` is a weak letter for "what to list" — is there a word-key that reads better
  (`k` kinds)?
