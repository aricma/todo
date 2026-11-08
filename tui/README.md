# lazytodo — the terminal client of aricma-todo

A small tool over the todo checkboxes in markdown files: it reads them, edits
them, and does nothing else. Point it at a folder and it finds the files; point
it at a file and it shows that one.

![lazytodo open on a repository: panels down the left, the file on the right, keys
across the bottom](../screenshot.png)

The paths and the collections down the left, the open file on the right with its own
line numbers, and the keys of whatever panel you are in along the bottom.

```sh
cargo install --path tui
```

```sh
lazytodo            # the TUI for the current directory
lazytodo path/to    # ... or this one
```

Every rule about what a todo line is lives in `lib/`, the core this screen is drawn
over. Commands live in [maskfile.md](../maskfile.md); the behaviour of the screen, one
decision at a time, is in [docs/rfc/](./docs/rfc/README.md).

## How the code is layered

`lib` owns the todo file (parse, check, write), `disk/` holds the file operations
it has no opinion on (make an empty file without overwriting, remove one, measure
one, list folders), `state/` is the machine the screen is a function of, `ui/`
draws that state, `terminal/` is the loop that joins them, `text.rs` holds every
word the user reads, and `helpers/` holds the small functions more than one layer
needs.

## The line grammar

There are two lines to know. The first is a todo in a markdown file:

```md
- [ ] (A) @dave fix you #worst-bug-of-the-century
```

and the second is a todo in any other file — a todo comment, read in place,
one line of code:

```rust
    // TODO: (A) fix the retry #infra
```

The word is one of four, **in capitals**: `TODO:`, `FIXME:`, `HACK:`, `XXX:`. A thing
that is broken is a thing to do, and the word stays in the line, so a `FIXME:` reads as
one on the screen without the tool having an opinion about it.

Between the word and the colon a note may name **who it is for** or **what it is filed
under**, in parentheses, and what is inside has to carry the sigil the body already uses:
`// TODO(@alice): ship it`, `// TODO(#home): pay rent`. That is the body's own `@alice` or
`#home`, in the body's own list, given once — and never a priority, because a priority is
written after the colon, where the body is. Without the `@` or the `#` the parenthesis is
text the line carries: `// TODO(bob): …` is a todo with nobody on it — it says nothing about
whether `bob` is a person or a label, so nothing is guessed and nothing is dropped. Three
spellings are not todos at all, and that is the limit of the rule: a space before the
parenthesis (`TODO (@bob): `), a parenthesis that closes on nothing (`TODO(): `), and one
that never closes.

```rust
    // FIXME(@alice): (A) #102 the retry is wrong
```

A `#` token is a **tag** whatever it looks like — `#102` and `#PROJ-404` are `#102` and
`#PROJ-404`, in the tag collection and in the tag count. A number somebody filed something
under is still a tag; there is no second kind of `#`.


The marker in front of it is the one the file's own language starts a comment with,
read from the extension: `//` in the C family (`.rs`, `.ts`, `.go`, . . .), `#` in
shells, scripts and their config files (`.py`, `.sh`, `.yml`, `Makefile`, `.gitignore`),
`--` in `.sql`, `.lua`, `.hs`. A file whose language the tool does not know accepts all
of them — `#`, `/`, `-`, `;`, `%`, `*`, `<!--` — and also a line with no marker at all,
because that is how a notes file writes a todo.

Between the marker and the word there may be other text, as long as it is inside the
comment: `// note: FIXME: the retry is wrong` is a todo. There may be **no quote or
backtick** anywhere in front of the word, and at least one character after the colon
that is not a space. Everything in front of the text — the indent, the comment marker,
the word, whatever the language puts there — belongs to the line and is never rewritten.

Capitals, a marker and the missing quote are all noise control: `// a file that no longer
holds a todo: …` is a sentence about a todo, `/// the todos are `TODO:` comments` is a
document about the rule, and `Doc::parse_comment("// TODO: x")` is a test fixture holding
one — all three were listed as todos before the rule had them.

The body after `FIXME: ` — or after `FIXME(@alice): ` — follows the same grammar as a
todo's body, which is why a priority is written **after the colon**: `TODO: (A) text`. A markdown file is never read
for comment todos, even inside `<!-- -->` or a code block; a code file is never read for
`- [ ]` lines. See RFC-016 for the grammar of a hit, RFC-019 for what the rule
deliberately misses, RFC-020 for the four words and the marker per language, and RFC-021
for the person or tag in the parentheses.

The rest of this section is about the first line, and holds for the body of the
second.

- `-`/`*`/`+`/`1.` — any bullet, kept as it is.
- `[ ]` / `[x]` — the state. `[X]` is read too; `--format` lowercases it.
- `(A)` … `(D)` — **the only positional rule.** Right after the checkbox, one
  letter, nothing else. A is important and urgent, D neither.
- `#tag` and `@assignee` — recognised **anywhere** in the body, any number of
  each. The word must be `[A-Za-z0-9_-]+`; anything else (`#café`, `bob@host.com`)
  is just text.

The body is stored exactly as it was read, so a toggle or a priority change
cannot disturb the spacing, the tags or anything the tool does not understand.

## Which files it opens

In a directory: files named `TODO*` or `TASK*` (any case) or `README.md`, ending
in `.md`, holding at least one todo — or at least one broken one, so `--check`
can find it — or nothing but headings, because a file made for the first todo is a
place to go and a list that hides it makes the file look absent. A readme with prose
in it and no todos is not about todos and is not listed. An explicit file argument is
always shown.

**The tree is what git does not ignore.** When the folder is inside a git work tree,
the file list is git's own — `git ls-files --cached --others --exclude-standard` —
so `.gitignore`, a nested one, `.git/info/exclude` and a global excludes file all
apply, and an ignored folder is never descended into, let alone read. Without git — a
plain folder, a tarball, a scratch directory — the walk falls back to skipping `.git`,
`node_modules`, `target`, `tmp` and dot-entries, ten levels deep. See RFC-017.

**Every text file in that list is read**, because a todo comment can be in any of
them, and a file is listed only when it holds at least one. The limits are what keep
that honest: a file over a megabyte is skipped unread, a file that is not UTF-8 or
holds a NUL byte is not a text file, at most twenty thousand files are opened in one
walk, and at most ten thousand lines are shown from one. Hitting a limit is a warning
in the file panel's footer, never a quiet short list.

**`h` widens it to every `*.md`** in the tree, and back. The convention above is a
promise a team makes about where its todos live; a team that never made it keeps
them in `backlog.md`, and this is how the tool reads a tree it does not know the
rules of. It is off by default because a large repository has a hundred files that
are not todos, and a list that hides the useful ones behind ninety useless ones is
a worse list. Pushing it through the same scan means the search, the collections and
the panel all widen together — one switch, one meaning.

## The frame

The top line is what this is and where it is — `aricma-todo` is the project, and this
is its screen, so the header says `lazytodo`, after the tick that says what it
is for:

```
✅ lazytodo · /Users/me/work/repo
```

One row, cut from the front when it is long (the tail of a path says more than its
head). Two sessions on two checkouts look identical otherwise.

The path is the **resolved** one, not the argument: `lazytodo .` shows
`/Users/me/work/repo` and not `.`, because the header is there to tell two sessions
apart and `.` says nothing about which checkout this is.

Every panel that draws document lines numbers them, in a dim gutter: the file's own
line number, so a todo seen on screen can be found again in the editor. That is the
todos panel and the file view both — the file view numbers its prose and headings
too, so a todo has the same number wherever it is read. The gutter is not part of the
todo: it stays unbanded, and the todo wraps in what is left.

## Two TUI layouts

Point it at a **folder** and the left column is the file list, the document's
**index** (its headings, for jumping around) and the session history, with the
file's own text on the right — so you can see the prose a todo sits under.
Point it at **one file** and it is the same window with one panel fewer: the
header, the index over the history in the left column, the todos of that file down
the right-hand side, the keys. Only the files panel is missing — there is one file,
so there is nothing to choose between — and the right-hand panel is the todos of
the file rather than a view of it, because a single file's prose is not what you
opened it for.

The index is always a section, even when the file has no headings at all: it is
where a heading gets made (`n` in it), so hiding it would hide the only way to make
one. Moving in the index (`↓`/`↑`, `g`/`G`) also moves the
selected todo to the first one after that heading, so the index is a way to
jump around the document; a heading whose section holds no todos hands over to
the nearest one before it.

**The one exception is a collection**: it has no headings of its own, so the index
would be showing the *open file's* — and moving in it moves the collection's cursor,
through sections the collection is not made of. While one is being read, the sidebar
is the file list and the history, and `tab` walks those two. Stepping back onto a
file row brings the index back.

`e` edits a todo where it sits: the row takes the text being typed and the
cursor moves into it, so nothing is typed into a bar at the bottom.

The top third of the panel is a margin: the selection walks down through it
before the document scrolls, then the viewport follows so the selection stays on
that third's last row. At the end the document stops scrolling and the selection
carries on down into the bottom third — how far depends on what follows the last
todo, since trailing prose keeps it higher. Nothing is padded with empty rows.

A todo is drawn as the line it is: indentation, bullet, checkbox and body, with
the bullet kept rather than hidden. The document is markdown and the person reading
it is reading markdown anyway — showing the prose and the fences but not the bullet
would be a screen that disagrees with the file. `*`, `+` and `1.` are shown exactly
as written, and the tool never rewrites one into another.

A heading wraps under its own text: the indent and the hashes stay on the first row and
the rest lines up past them. A history entry does too, but only when you ask for it —
`h` in the history switches between one row cut with `…` and the wrapped shape, and one
row is the default, because the trail is what that panel is for and three long write
names push a session's history off the screen. Both panels scroll to the cursor when an
item pushes it out of view.

A todo that does not fit wraps onto continuation rows, indented under its own
body — one displayed column past the body's first character — because half a todo
is not a todo. Prose does not wrap: a paragraph that reflowed would push the todos
the panel exists for off the screen, so it stays one dim row clipped with `…`.

`Tab` stays in the sidebar and loops it — inside the file view it does nothing,
because the view is not a sidebar section: `→` crosses into it and `←` is the
only way back, to the section you left. Checking a todo needs the
view: `x` does nothing in the file list or the index, so a key pressed while
picking a file can never change a todo.

The status bar is two rows: the keys every panel shares, then the keys of whatever
has the keyboard right now — the active panel's, or the open modal's. Navigation
comes first in both, and there is nothing else there: which panel is active is said
by its frame's dot, and the counts belong to the file they count.

The counts of the open file (`3/7` done) sit at the right end of the bottom border
of the panel that shows those todos, in the file view and in a single-file session
alike: a number belongs to what it counts, and the status bar belongs to the keys.

Each section is its own framed panel with a rounded border and its title in the
border, numbered so the number is the key that jumps there; the one with the cursor
carries a dot in front of its name as well as the bright border — a colour alone is
easy to miss on a screen of framed panels, a dot is not — and the rest are dim.

### What being selected looks like

One idea, everywhere: the row under the cursor wears a **green band** — a dark
green background (`Rgb(0, 45, 22)`, one constant in `ui/consts.rs`) running the full
width of the panel's text, with the text left as it is. A band that stopped at the last
character would read as marked-up text; a filled row reads as "the cursor is on
this line". A file, a heading, a history entry, a todo and a search result all
draw it the same way, so there is nothing new to learn from panel to panel.

The one thing the band does not cover is a **gutter**: a line number, a history entry's
dot and dependency lanes, a heading's `##` all stay on the panel's own background, because
a highlight over a marker reads as a highlighted number. The band starts at the text and
runs to the panel's edge — less one column, which belongs to the scrollbar whether or not
one is showing, so the bar never has to repaint its own background.

It is not reverse video. White-on-black flips the whole row and flashes the eye on
every arrow key; a band says "the cursor is here" and leaves the text legible. No
row carries an arrow or a marker either: one mark for one meaning.

The band is the only mark, and nothing is inserted around the selected row: the
row above it stays its neighbour, so moving the cursor never makes the list jump by
two rows.

`t` is the one appearance setting, and it is about **line height**: one blank row
between rows, everywhere. Off by default, because a terminal has no half-line — the
smallest step is a whole row, which halves what fits in a panel. On, it is a list
with room to breathe; off, it is the dense one. Nothing about the selection changes
either way.

### The file panel: files, then collections

```
▸ crates/lib/TODOs.md  3/7      the file the cursor is on
  web/src/TODOs.md  0/1         where it lives, grey, and its own name, not
  web/Tasks.md  0/4
```

Flat, one row per file, in the order a file viewer shows: folders first, then files,
each compared without regard to case (`crates/lib/TODOs.md`, then `web/src/TODOs.md`,
then `web/Tasks.md`, then `README.md`). The cursor walks the list exactly as it is
drawn — the order is the list's, not a second sort somewhere else. The folder is
*where* a file is, not something you can act on, so it is named inside the row
instead of taking a row of its own, which also means a folder never pushes the files
it holds down the panel.

After the files come the **collections**: one row per `#tag` and one per `@person`
mentioned anywhere in the tree, each with how many todos carry it, each in the colour
the todo panel gives that kind of name.

```
│ web/TODOs.md  1/2  #3  @1  h1  │
│ README.md  0/4                 │
│ #home  12                      │
│ #infra  3                      │
│ @dave  5                       │
```

A collection is **not a file and not a folder**: a folder is *where* a todo lives, a
collection is *what it is about*, and the two cut across each other. So nothing is
stored for it — the scan says which files mention a name and how often, and the
todos are gathered from those files when the collection is opened. `#Home` and
`#home` are two collections: a tag is a word someone wrote, not a normalised key.

`v` cycles what the panel shows — both, tags only, people only, files only — because
a repository with forty tags would otherwise double the whole list.

A file row carries what the file is about: `1/2  #3  @2  h1` — done/total, the
**distinct** tags it mentions in the todo panel's cyan, the distinct `@people` in its
magenta, and how many writes this session holds for it in the warning orange. A tag
written on nine lines is one tag: this is a summary of the file, not a tally of the
occurrences. Nothing is shown for a count of zero — a row of `#0 @0` is noise. The history is per file, so this is how you tell which files have one
without opening them; a file with nothing recorded says nothing.

A folder with no todo file in it is not listed at all: the panel lists what the scan
found, not the disk. An empty folder, or a `README.md` with nothing to do, is out.
The `n` field is where the folder structure is discovered, since typing a path there
offers the folders that actually exist.

The list scrolls with the same rule as the file view: the selection walks down the
panel's top third before the list moves, and clamps at the end.

### `d`: deleting a file

`d` in the file panel removes the selected file from the disk. A file with nothing in
it goes without a question — there is nothing to lose — while a file that holds
anything is asked about first, in a modal over the screen, because a line in the
status bar is easy to miss:

```
╭ delete file ─────────────────────────╮
│ a/TODOs.md                           │
╰─────────────── esc cancel · Y delete ╯
```

Only a **capital `Y`** deletes, and **any other key cancels** — `esc`, `enter`,
`n`, a lowercase `y`, a key pressed by accident. There is no second answer to
learn: the question has one key that means yes, and everything else means no, so
nothing opened by mistake can destroy anything. A todo takes the same capital as a
file even though a todo line can be undone from the history: one rule for the one
question, and the deliberate key is the point.

Afterwards the list is rebuilt from the disk and a neighbour opens. The writes
recorded against the deleted path are dropped with it: their lines no longer exist,
so nothing could ever undo them. Deleting the last file ends the session — there is
nothing left to be about.

### `n`: making a new todo file

```
╭ new todos file ──────────────────────────╮
│ ./web/src/TODOs.md                       │
│   web/src/inner                          │  ← offers, marked with ↑/↓
╰────────────── esc cancel · enter create ─╯
```

The field shows the whole path it will write, not a fragment of it: `./`, then what
is typed, then `/TODOs.md`. The `./` and the `/TODOs.md` are grey because they cannot
be typed — the file always gets the one name the scan looks for, and the path is
always relative to the tree the tool was opened on — and the cursor lives only in
the middle. So the answer to "what is this about to create?" is on the screen, and it
is the same string that gets written.

The middle starts in the folder of the file that was selected: empty at the root,
where the path reads `./TODOs.md`, and `web/` inside `web/`, so typing continues
*inside* the folder already open.

What the folder offers is read as it is typed: the folders one level down that match
what has been typed so far, up to five. `↑`/`↓` mark one, and typing takes the marked
one and continues from it — marking and typing on is one motion, not two. A path
that already *is* a folder offers what is inside it.

`Enter` is blocked while the field says why the path is no good, and the field says
so itself, above the controls: `TODOs.md` is already there (nothing is ever
overwritten), or the path walks out of the tree the tool was opened on. `Esc` closes
the field and leaves the disk alone.

The file is made with the heading everybody wrote by hand anyway — `# <folder> todos`,
naming the folder it was made in — and opened. A file with only a heading still holds
no todo, and the scan lists files that hold one, so it is listed as `0/0` until the
first todo is written; on a reload it drops out of the panel until then, which is
honest — it is not a todo file until it has a todo.

### A collection

**The collection is drawn as soon as its row has the cursor**, the way a file row's
document is: moving onto `#home` shows every todo that carries it, while the keyboard
is still the file panel's. `→` then only hands the keyboard over, and `←` gives it
back — the collection stays on screen, because the row that is about it is still the
one highlighted. Stepping onto a file row shows that file again.

The panel on the right is the collection itself: a heading it never writes to any
file, then the todos **grouped under the file they live in** — the path as a heading
of its own, over its own lines, with a blank row between one file's todos and the
next. A file is where a todo came from, which is one fact about all of them: as a
prefix on every line it was forty copies of the same sentence, and it pushed the
todos out of the column they are read in.

Each line is the line: indentation, bullet and checkbox as the file has them, and the
file's own line number in a dim gutter, so a todo seen here can be found again in the
editor.

```
╭ ● [0] #home todos ───────────────────────────────╮
│ # tag home todo collection                       │
│                                                  │
│ web/TODOs.md                                     │
│  4 - [ ] (A) pay rent #home                      │
│ 12 - [x] buy milk #home                          │
│                                                  │
│ notes/TODOs.md                                   │
│  2 - [ ] call the plumber #home                  │
╰────────────────────────────────────────── 2/3 # 1╯
```

- **Order is file order.** Files as the panel lists them, and inside a file the order
  the todos are written in. A collection is read as "what do I owe about this", and
  re-sorting it by title would be the tool's opinion rather than the author's.
- **Every row knows its file**, because every edit goes there: checking a todo in a
  collection is a write to the file that todo came from, with that file's staleness
  check and that file's conflict question. Editing it with `e` works the same way,
  and the typed text is drawn on the row it is editing, underline and cursor
  included — a panel that hid what you were typing would be a panel you cannot use.
- **The history panel follows the cursor**: it shows the history of the highlighted
  todo's file, and says so in its title (`history · web/TODOs.md`).
- **`enter` goes to the file.** A collection is a way of reading files, and `enter`
  on a row ends the reading: the collection closes, the file panel's cursor moves to
  that file's row, the file's own headings come back with the index on the heading
  over the todo, the view scrolls to it, and the history shows that file's writes.
  It is the same jump a search hit makes — one function, two ways in.
- **`x`, `e`, `A`–`D`, `d`, `c` work**; **`n`, `h` and moving do not** — a new
  todo has no file to go in, the scan's switch belongs to the panel that lists
  files, and a collection has no order of its own to move in. `n` and moving do
  nothing and say why; `h` is not offered and does nothing, because this panel is
  for reading todos.
- **The panel scrolls to whatever the cursor is on**, collections included: the
  selection walks down its top third before the list moves at all, and `g`/`G` reach
  the first and last *row* — a tag at the bottom is as reachable as a file at the top.
- **Order is size.** Tags and people are each listed biggest first — a list of forty
  tags is read to find the busy ones, and a name is easier to find with `f` than by
  scrolling. A tie falls back to the name, so rows do not swap places between scans.
- The tally is `done/total` for the collection and nothing else: it *is* one tag, so
  counting its tags would be counting itself.

### A comment todo file

A code file with a `TODO:` comment in it is listed in the file panel like any other
place to go, and opening it shows it with the file's own line numbers. What is
different is what the screen shows and what the keys can do.

```text
╭ ● [0] src/retry.rs ─────────────────────────────╮
│  41     if job.attempts > 3 {                   │
│  42         // TODO: (A) back off before retry  │
│  43         return Err(Error::TooMany);         │
│  44     }                                       │
│  45     job.run()                               │
│   …                                             │
│ 201 fn teardown() {}                            │
╰──────────────────────────────────────────── 0/2 ╯
```

- **One line of code above a todo and three below it**, and one `…` for everything else.
  The line above is the block the note sits in and the lines below are the change it asks
  for, because a note is written *above* the code it is about — every developer writes it
  in that order. Everything outside that window is folded, at the start and end of the file
  too: a file does not open on twenty lines of imports to reach its first note. A run of a
  single line is shown rather than folded, because the `…` costs the same row and saves
  nothing. Every line shown keeps the file's own number, and the `…` has none. See
  RFC-018.
- **`e` edits the text after the severity word's colon** and nothing else: the indent,
  the marker, the word and whatever follows on the line are the file's, and stay
  byte-identical.
- **`c` copies it as an OPS document**, exactly like a todo in a document — which is
  the way out: paste it into a todo file and it becomes a todo you can check, move and
  keep. That is the only way to "do" a comment todo, since a comment has no checkbox.
- **`A`–`D` and `0` write a priority after the colon** (`TODO: (A) …`), **`d` deletes
  the whole line** with `Y` to confirm.
- **`x`, `space`, `n`, `K`/`J` and `ctrl+↑/↓` are not offered, and say why if pressed**
  anyway: a comment cannot be checked, the tool does not know where in the code a new
  comment belongs, and moving it would take it away from the code it is about. The
  status bar draws only the keys that work, which is how a reader learns the
  difference without a page about it.
- **A collection follows the row.** A `#tag` collection can hold a comment todo from
  one file and a document todo from another; the keys offered are the ones the
  highlighted row's file allows.

## Keys (the TUI)

`?` opens the whole key map in three columns — `general` and `typing` for the keys
every panel shares and the ones a field takes, then `files`, `index` and `history`
down the middle, then `todos` and `collection` for the panels that edit one. The panel
you are in carries the same dot its frame does. `?`, `esc` and `q` close it.

It is **one table** — `BINDINGS` in `ui/hints.rs` — and both the page and the status
bar are drawn from it, so a key cannot be on the page and missing from the bar, or
offered in the bar and unknown to the page. Each row says what it needs: the panels
it works in (`Where`, as bits, because most keys work in several) and whether
something has to be under the cursor. `x`, `d`, `e`, `A`–`D` and a collection's
`enter` all need a row to act on; `n` does not. The bar therefore drops the ones that
would do nothing — an empty file offers `↑/↓ nav · n add · c copy`, and `enter`
appears on a collection's row only while there is one.

Inside a field the page lists only the three keys that are not obvious — `esc`
cancels, `enter` submits, `ctrl+u` empties the search. `←`/`→`, `home`/`end`,
`backspace` and `delete` do what they do in every text field ever written, and each
field's own border carries the word for `enter` (`enter add`, `enter save`,
`enter create`) rather than the page's `submit`, which is the same key with the
name that fits whatever line is being typed.

The page is as wide and tall as its content: a column narrower than its longest row
wraps a sentence onto a line of its own, which is what the old three-fifths columns
did. Each column is exactly as wide as its widest row, there are two cells of air
between them, and two columns inside each border (the frame's own padding and one
more) so the first key does not touch the frame. At eighty columns the page is 77
cells, so nothing wraps.

| key | what it does |
| --- | --- |
| `Tab` / `Shift-Tab` | next / previous sidebar section; loops, never crosses to the right. The status bar names the panel it would reach (`tab index`) |
| `0`–`3` | the panel whose title carries that number: `[0]` the right-hand one (the todos panel, the collection, or the file view), `[1]` files, `[2]` index, `[3]` history. `0` for the panels a person works in most is what lazygit taught everyone. Fixed numbers, so they survive the layout changing, and a panel that is not on screen takes no number |
| `→` | step into the view: the file, or the collection the cursor is on (the hint names it, and the panel already shows it) |
| `←` | cross back to the section you left (the hint names it) |
| `↓` / `↑` | move inside the focused panel: file, heading, todo, history entry |
| `x` / `Space` | check the selected todo, in a todo panel (the view, the todos list, a collection). In the history `x` does nothing and `Space` alone undoes/redoes the entry: a write is not a todo and has no checkbox |
| `d` | delete: the todo in a todo panel (then a capital `Y`; any other key cancels); the whole file in the file panel (the same capital, always asked); in the history, undo that write and forget the entry, without asking |
| `enter` | a collection's row: open the file the todo lives in — its view, the cursor on that todo, the index on the heading over it, the file's own history. The same promise `enter` makes on a search hit |
| `n` | new: in the file panel a todo file (type a folder, `TODOs.md` is made in it with a `# <folder> todos` heading and opened); in a todo panel a todo; in the index a heading (`## later` keeps its level, `later` becomes `# later`), added at the end of the file |
| `e` | edit the selected todo's text, on its own row |
| `K` / `J`, or `Ctrl+↑` / `Ctrl+↓` | move the selected todo up or down: inside a chapter it swaps with the nearest todo that way; across a heading it joins the chapter's own block of todos on the other side (RFC-012) |
| `A`–`D` | set the priority; the letter a todo already has clears it (`()` → `(A)` → `()`). That is the only way to clear now: `0` used to do it as well, and gave the key up to the todos panel's number |
| `g` / `G`, `home` / `end`, `shift+↑/↓` | first / last row of the section in the focused panel. Three spellings on purpose — see "Terminals that eat the modifier" |
| `Opt+↑` / `Opt+↓` | the previous / next sidebar section, like `shift+tab` / `tab`; in the file view, where there is no sidebar, it moves the todo |
| `h` | show less of the list, in the two panels where a list can be too long. In the file panel: every `*.md` in the tree, or only the files named for todos — see "Which files". In the history: one row per write, cut with `…`, or wrapped — one row by default. The bar says which way it goes (`h unhide *.md` / `h hide *.md`; `h unhide` while the labels are one line, `h hide` while they are wrapped), because a switch that names what it just did reads as its opposite |
| `c` | copy: in the file panel the selected file's path or the collection's own word (`#home`, `@sam`); in a todo panel the todo's line (markdown and all); in the index the heading's text. Not in the history, which is a log rather than a thing to paste |
| `v` | cycle what the file panel shows: files and collections, tags only, people only, files only |
| `t` | air around the selection, or none — see "What being selected looks like" |
| `esc` | dismiss the message under the thing it is about; the next key does the same |
| `?` | the keys page: every binding, grouped by the panel it works in, the panel you are in first |
| `q` | quit the app, from anywhere it is not a letter being typed |
| `ctrl+c` | quit, anywhere — the one key that works in a text field too |
| `f` | open the search popover (never on top of another modal). `ctrl+f` is the same key where a terminal forwards it, and it works inside a field too |
| `ctrl+u` | empty the search query; nothing while the popover is closed |

There is no reload key: switching to a file reads it fresh, and a write by anyone
else is picked up within a second, so there is nothing a reload would add.

### Terminals that eat the modifier

Modifiers on arrow keys are the least portable thing a terminal does. macOS
Terminal.app does not report them at all: `shift+↑` arrives as a plain `↑`, and
`opt+↑` arrives as `Esc Esc [ A`, which reads as the `Esc` key followed by two
letters. Nothing in the app can tell those apart from what they look like.

So every modified-arrow binding has a spelling that cannot be eaten:

| does | sure things | if the terminal reports modifiers |
|------|-------------|-----------------------------------|
| first / last row | `g` / `G`, `home` / `end` | `shift+↑` / `shift+↓` |
| move the todo | `K` / `J` | `ctrl+↑` / `ctrl+↓` |
| next / prev section | `tab` / `shift+tab` | `opt+↓` / `opt+↑` |

The status bar carries the left-hand column for the same reason: it is the row a
person reads while working, and offering `ctrl+↑` to a terminal that cannot send it
would be offering a key that does nothing. The other spelling is on the keys page.

Kitty, WezTerm, foot, Ghostty and iTerm2 report them (the app asks for the kitty
keyboard protocol at startup, which is what makes `CSI 1;3A` arrive instead of
letters). `?` on the keys page shows **the last key as the terminal described it**,
which is the quickest way to find out which half of the table you are in.

### One modal at a time

A modal owns the keyboard. While one is open — the add field, the edit row, the
new-file field, the search popover, the delete question, the keys page — nothing
else is bound: no panel keys, no checking, no second modal. `f` while the add
field is open does nothing at all, because a search opened over it would strand the
half-typed todo behind a popover with no way back to it; and `n` or `?` while the
search is open are letters in the query. `esc` leaves, and the keys that were there
before come back exactly as they were.

### `q` quits, `esc` cancels

`q` never means "close this". It quits the app, and whatever was open is simply gone
— a delete it interrupted does not happen, a keys page it interrupted says nothing.
The one place `q` is not a command is where text is being typed: in the add field,
the edit row, the new-file field and the search query it is the letter `q`, because
a key that quits cannot also be a letter a todo needs. `esc` is the only way to
close something.

## Search

`f` opens a popover over the screen: type, and the todos whose text matches
are listed under the query, best match first. Only todos are searched — never
headings, never prose — and every file the scan found is included.

The match is fuzzy and case-insensitive: `pyrnt` finds `pay rent`, and a match
that starts a word or runs contiguously scores above letters scattered through a
longer line. `↑`/`↓` pick a result, `enter` opens its file, selects the todo and
moves the index to its heading, and `esc` closes the popover.

Its own border carries its own keys, like every other modal: `esc cancel`, and
`enter jump` only while a hit is selected — a key that would do nothing is not
offered. The bar says the same two words, for the same reason.

The box is **as tall as it needs to be** — the query, then one row per hit — up to
two thirds of the terminal, where the list starts scrolling instead of growing. A
search that found one todo saying so in a box the size of the screen is a modal
pretending to be busy. The title says how many matched (`search · 12 matches`),
because a window cannot say how much of a list it is showing, and the list scrolls
to whatever `↑`/`↓` has selected, the same rule every other list follows.

The query survives closing, so reopening re-runs what you typed; `ctrl+u` clears
it. Not `x` — the popover is a text field, and a key that erases what you typed
would make searching for `xfce` impossible.

`f` is a plain letter, not `cmd+f` or `ctrl+f`: a terminal keeps the command key for
its own find and often the control one too, and a key nobody can press is not a key.
Inside the query it is the letter `f`, like every other letter — the popover is a
text field, so the key that opened it types into it.

### Where an add and a move land

In a todo panel, `n` does two things at once. It writes the **row first**: an empty
`- [ ]` goes into the file directly under the one that was highlighted — same indent,
same bullet — and the highlight moves onto it. Then it opens the field, a small frame
over the middle of the screen — a bit under half the width, as tall as the text in it
needs, because what does not fit wraps instead of being cut off — with its keys written
into the bottom of the frame so they cost no row. The row says **where** the todo goes,
the field says **what** it says, and neither has to be guessed: the file's own numbering
counts the waiting line, the view shows it where it will be, and it is drawn as `- [ ] …`
so an empty checkbox reads as an invitation and not as a broken todo. The dots are drawn,
never written — the file gets the title.

What is typed is coloured as it is typed, so a `#tag` turns cyan and an `@who` magenta
the moment they are spelled right. `Enter` writes those words into the row that was
waiting for them. `Esc` takes the row back out and puts the highlight on the row it came
from, so the document and the cursor are what they were. Nothing reaches the disk before
`Enter`, so a cancelled add — or quitting mid-add — is not a line on anyone's file. A
file with nothing selected appends at the end. Text the rules refuse — a todo with no
title — keeps the row and the field open and says why, because throwing away what was
typed is never the better answer.

A new **heading** is the same field without a row behind it: the index lists headings
that exist, so there is nothing to prepare until the heading is made.

A move that cannot happen says why, in orange, rather than doing nothing quietly:
`nothing below it to swap with` at the end of the file, and the same words above
when the heading above is the file's first line, where there is no section to join.

`Opt+↑`/`Opt+↓` exchange the todo's line with the nearest todo line in that
direction, and touch nothing else. At startup the tool pushes the **kitty keyboard
protocol** (`CSI > 1 u`, popped again on the way out and from the panic hook), which
is what makes a terminal send the modifier with the arrow instead of a sequence
nobody can decode — macOS Terminal.app sends `ESC ESC [ A` without it, and that
reads as the Esc key followed by two letters. A terminal that does not know the
sequence ignores it, so nothing is lost where it is unsupported. Which modifier arrives with the arrow is the
terminal's business — `alt`, `meta`, `super`, `shift` and `ctrl` all move the todo,
because none of them means anything else here — and `K`/`J` do the same thing for a
terminal that swallows the modifier altogether. Prose and blank lines are stepped
over and stay where they are, because a note that followed its todo would start
describing whichever todo landed next to it.

A **chapter** is its heading, whatever the chapter has to say — prose, a table, an
image, a code block — and then its todos, together at the end (RFC-012). A todo that
crosses a heading joins the chapter on the far side **in its block of todos**: after
the last todo when the move is up, before the first when it is down. The chapter's own
text is stepped over with the heading, because a todo that came down between the
introduction and the todos would be split off from the ones it belongs with. A chapter
with no todos has no block to join, so the todo lands at that chapter's end and starts
one. One heading per press: the key does not hop again to find a bigger crowd.

One lift and one landing is one edit, so one undo puts the todo back on the line it
started on. The heading above may be the file's own title, with nothing above it: there
is no chapter to join, so the move says `nothing above it to swap with` rather than
filing a todo above the title.

### Copying

`c` puts the selected thing on the clipboard. A file gives its **path**, a heading its
**text**, a collection row the **word** it is (`#home`, `@sam`) — those are strings, and
a person uses them as strings. A **todo** gives an **OPS document**: its title, its
status, its priority, its tags and its people, as JSON, because a todo is data and a
line is a rendering of it. That is what makes a copy survive the trip — and what lets a
document from somewhere else (an `--as json` export, another tool, an agent) come back
the same way (RFC-014).

**A paste puts the todo back as a line.** Bracketed paste is on for the session, so the
clipboard arrives as one string instead of a hundred keystrokes. In a field it is text
and goes in at the cursor. In a todo panel or the file view it is read as an OPS
document and written out as todo lines **under the highlighted todo** — `(A)` tokens,
`#tags`, `@people` and checkboxes as this file writes them — and the header says
`pasted 2 todos`. The file never receives the document itself, so nobody has to notice
that the clipboard was JSON. A paste that is not OPS says so and writes nothing: a line
copied out of an editor is exactly the guess this avoids.

**The feedback is a success, in the header.** `copied - [ ] pay rent #home` appears
green at the right-hand end of the top line, where nothing on screen has to move over
for it, and the next key takes it away like every other message. It never takes more
than half the row, so the tree stays readable; if the copied line is longer than that
it trails off with `…`.

That is the one thing the header says, and it says it because a copy is the only
outcome with nothing on screen to be about — every other message is drawn beside the
thing it explains.

The clipboard is reached with `OSC 52`, the one a program on the other side of a pty
can write to. Not every terminal implements it — Terminal.app ignores it silently —
so on a macOS session that is not itself over ssh the text also goes through
`pbcopy`, which fills the system clipboard whatever the terminal does. Over ssh only
the sequence goes out, because `pbcopy` on the far machine would fill the wrong one.
The message says *what* was copied either way.

## Limits

A repository is unbounded, so a scan is not. `lib/src/limits.rs` holds the three
numbers, and the screen reads them from there:

| limit | value | what happens |
| --- | --- | --- |
| `MAX_DEPTH` | 10 folder levels below the path given | a folder deeper than that is not entered, and the scan says which |
| `MAX_FILES` | 1 000 candidate files | the walk stops and says which folder it stopped at |
| `MAX_LINES` | 10 000 lines in one file | the file is listed, dimmed and marked `too long`, and never opened |

Warnings are not silent. The file list prints them in orange at its bottom — the
walk is what the list shows, so that is where its limits belong — each one a line
that says its level:

```
[warn] 1 folder is deeper than 10 levels, not searched (first: …/web/vendor/css)
```

A blocked file is dimmed and marked `too long`, and `↓` steps over it rather than
stopping on it. A folder whose every todo file is
blocked refuses to open at all.

## Where messages go

Two different things, and the words are worth keeping apart:

- a **hint** is the reminder of a key — `tab index`, `f search`. No tone: it
  is simply true until the layout changes, and it lives in the status bar, which is
  the keys and nothing else.
- a **message** is about something that just happened — a rule being explained, a
  write refused, a query that found nothing. It carries a tone, because "worth
  knowing" and "did not happen" must not look the same.

A message goes **under the thing it is about**, and it goes away by itself. The
tone decides the colour, the way a log level would: dim for information, orange for
a rule, red kept for a write that did not happen, and green for the one thing that
did happen — a copy, which has nothing on screen to be about and goes in the header
instead.

| message | tone | where | when it goes |
| --- | --- | --- | --- |
| `enable the write below it first` | warning | under the history row, while that row is selected | the moment the row below is switched back on, or the cursor moves |
| `that line changed since then — it reloads on its own` | warning | under the history row that refused | the next key press |
| `… more than 10000 lines, not opened` | warning | under the file list | the next key press |
| `a todo needs a title` | warning | in the add field, under what was typed | the next keystroke |
| `no todo matches` | info | under the query, in the popover | the next keystroke that changes the query |
| why a search could not run | error | under the query, in the popover | the next keystroke that changes the query |
| `… changed on disk` when a save is refused | error | under the file list, until the save goes through | — |
| `copied - [ ] pay rent #home` | success | the header, right-aligned | the next key press |

Nothing that only explains a rule wears red: a todo that cannot be switched on
yet, a file too long to open, a title that is missing — none of them is a failure,
and a message for them in the colour of one teaches a person to ignore red. A
search that finds nothing is not a warning either: nothing is wrong with the query,
so it is information.

**A message about a row lives on that row.** The history entry holds its own, which
is why a note cannot end up under the wrong one when a row is forgotten in between;
the message about a panel lives on the app, and the panel that is on screen draws
it. Nothing carries an index into somebody else's list.

The first one is not stored at all: it is derived from the state, so it is already
there when the cursor lands on a row that cannot be switched on, the way a form
validates a field it has just given focus. Everything else is a report of
something that happened, and is cleared by the next key press so that no note is
carried around after it stops being true.

### When the file changes under you

The tool never guesses. Every write is checked: if the file is not what it was when
it was read, the save is refused — but refusing alone would leave an edit that can
never be written, so the refusal becomes a question:

```
╭ changed on disk ─────────────────────────────────╮
│ web/TODOs.md changed on disk                     │
│ ──────────────────────────────────────────────── │
╰──────────── Y keep mine · t take theirs · q quit ╯
```

- **`Y` keeps yours**: your document is written over the file, atomically, and that
  becomes the new baseline.
- **`t` takes theirs**: the file is re-read and your untouched edit is dropped,
  along with the history, which was anchored to text that no longer exists.
- **`q`** quits, and the edit goes with it.

Nothing else dismisses this one. Every other question can be cancelled safely —
cancelling a delete deletes nothing — but cancelling a conflict would have to pick a
side, so the only way out is to choose.

An external write that arrives while nothing is being typed is not a conflict at
all: the file is simply re-read, because there is nothing of yours to lose.

## Staying current

The open file is re-read when something else writes it: the loop polls the file
every 0.75 s (a `stat` and a small read), and when the bytes differ from what was
loaded, the screen reloads and says `reloaded from disk`. That reload forgets the
session history, because every entry is anchored to the text that just changed.

Two guards: a write of the tool's own updates the baseline, so its own saves are
never mistaken for an external change; and a refresh is skipped while a prompt is
open, so nothing replaces the document under someone who is typing. There is no key
that forces a reload: the poll is cheap and nothing is gained by asking. `cmd+r`
could not be one anyway — a terminal hands that to the terminal, not to the program
inside it.

## History

Every write is remembered **for this session only**, per file, newest first. A
row says what was done and to which todo — `check · pay rent`,
`priority B · ship it`, `delete · old idea` — in front of a box that holds the
state: `[x]` in effect, `[ ]` undone. `x` acts on the selected row: on a write in
effect it undoes it, on an undone one it redoes it — once each way.

A write that added or deleted a line replays **literally** in both directions:
undoing a delete puts that line back, redoing it deletes it again. The same line,
never a second copy of it.

### Dependencies

A write **depends on** the nearest older write to the same line that it undoes —
not simply whatever came before it. A `priority clear` depends on the `priority B`
it cleared, and a `check` and an `uncheck` depend on each other; a `check` in
between two priority writes depends on neither of them.

```
╭──● priority clear · write the README
│╭─● uncheck · write the README
╰┼─● priority B · write the README
 ╰─● check · write the README
```

`╭` leaves a row downwards, `│` carries on, `╰` arrives and runs right into the
write it lands on, and `┼` is a lane running through a run. No arrowheads: the
corner that turns into the row is already saying where the line ends.

Each chain gets a lane of its own in the gutter, and every row is padded to the
gutter's width, so no label is pushed sideways by the graph. A dependency line
crosses unrelated writes without touching them, and a chain of three keeps one lane
from end to end.

Each entry keeps the write it describes, so toggling a row one way and back
leaves the graph exactly as it was: the lanes are drawn from what each write
changed, not from the state the file happens to be in.

Undoing a write undoes the writes that hang on it — and nothing else:

| press `x` on | undone | left alone |
| --- | --- | --- |
| `priority clear` | itself, and `priority B` under it | the checkbox chain |
| `uncheck` | itself | its parent `check`, the priority writes |
| the oldest `check` | itself and the `uncheck` that hangs on it | the priority writes |

Redoing goes the other way: a write can only go back on top of the one it undoes,
so `x` on a row whose dependency is undone refuses with `enable the write below it
first`. Nothing is ever left half undone: the whole step is tried on a copy of the
document first.

`d` goes further: it undoes the write (its dependents with it) *and* forgets the
row, with no confirmation, because the thing being deleted is the row itself and
nothing new is recorded. A row that is already undone is simply forgotten.

### Why an undo can still refuse

Each undo is re-derived from the line as it reads *now*, so an unrelated write in
between — a priority change between a check and its uncheck — cannot block it.
What must still hold is the part of the line that write owns: the checkbox it
toggled, the priority it set, the body it wrote. When that no longer reads as it
was left, the undo refuses with `that line changed since then` instead of writing
the wrong line. Quitting forgets everything, which is why there is no undo file
and no journal.

## OPS

`c` on a todo puts an OPS document on the clipboard rather than the bare line
(RFC-014), built through the reference `ops-lib`. The title is the line minus its
priority token, the tags and assignees stay inside it because that is how the line
reads, and the parsed extras ride along as `metadata`:

```json
{ "id": "md-1",
  "status": "open",
  "title": "@dave fix you #worst-bug-of-the-century",
  "metadata": { "priority": "A",
                "tags": ["worst-bug-of-the-century"],
                "assignee": "dave" } }
```

Which is why the document survives the trip: what is on the clipboard is a
rendering of the line, and a paste reads that rendering back into lines. A todo can
leave for an agent and come back without being retyped.
