# RFC-001: Panels and their names

- **Date:** 2026-10-02
- **Related:** [RFC-002](RFC-002-numbered-lines.md), [../glossary.md](../glossary.md)

## Context

A session shows up to five things at once, and three of them can show todos. As long
as every one of them was called "the todos panel", a sentence about the screen could
not be answered without asking which panel was meant: an item that said *add line
numbers to the todos panel* was already true of two panels and false of a third, and
every reader took it to be about a different one.

Two panels really do show todos. In a single-file session the right-hand column is
the **todos panel**, and its source is that one file. Opened from the files panel, a
tag or a person is also drawn as a **todos panel**, titled `#home todos`, and its
source is every file under the root. In a folder session, a file's todos are shown
inside the **file view**, which draws the whole document — prose, headings and todos,
in the file's own order.

Those two things are not the same panel doing two jobs. The todos panel shows *the
todos of one thing*, with their line numbers and their stats, and nothing else. The
file view shows *a file*, and a todo in it is one line among prose.

## Behaviour

1. The panels **MUST** be called, and titled, exactly: **files panel** (`files`),
   **index panel** (`index`), **history panel** (`history`), **todos panel** (`todos`
   or `<collection> todos`), **file view** (the file's path). These five names are
   the names in `docs/glossary.md`, and the glossary is the list an argument is
   settled with.
2. A panel **MUST NOT** be named for where it sits. No "left panel", no "middle", no
   "right side": the layout moves between a folder session and a single-file session,
   and the names do not move with it.
   A single-file session **MUST** be the same window as a folder session minus the files
   panel: the sections down the left column (the index above the history) and one panel
   down the right. It **MUST NOT** stack its sections on the right instead — that was the
   shape it had, and it made one file look like a different program from two.
   `→` and `←` **MUST** cross between the two columns in both layouts, and come back to
   the section they left: the right-hand panel is the todo panel of a single file, so the
   pair of arrows is the same pair, and the bar **MUST** name the destination (`→ todos`,
   `← index`) rather than leaving a person to find out.
3. The **todos panel MUST** draw the todos of exactly one source, and **MUST** say
   which in its title: `todos` for a file, `<tag> todos` / `@<person> todos` for a
   collection. A source is a file or a collection, never both.
4. Anything that is true of the todos panel **SHOULD** be true of it for either
   source: numbering (RFC-002), wrapping under the body, the band keeping a tag's
   colour, the stats on its frame, the edit cursor inside a row — and **scrolling**,
   which is the same arithmetic as the file view's and the collection's: a height per
   row, an offset that keeps the cursor where a reader expects it, and a thumb when the
   rows do not fit. A panel that draws every row into its rect clips its list and lets
   the cursor walk off the bottom, which is what the todos panel did until it caught up.
5. The **file view MUST** show the whole document, in file order, with prose — it is
   the one place the prose around a todo can be read. It **MUST NOT** grow
   collection rows or todo stats: it is not a panel about todos.
6. A **collection MUST NOT** be shown with an index panel: the index would be the
   open file's headings, and moving in it would walk another file's sections.
7. The wording of a message **MUST** name the panel it is about, in these words,
   wherever the panel it is about is not the one on screen.
8. A panel's title **MUST** start with the number of the key that jumps to it: `[0]`
   for the right-hand one (the todos panel, the file view, or a collection, which take
   that place in turn), then `[1]` files, `[2]` index, `[3]` history. `0` is the panel
   a person works in most, which is what lazygit taught everyone, and it is the one
   number every other layout of this screen will keep. The numbers **MUST NOT** follow
   the layout: a number that moved between a folder session and a single-file one would
   be a different key in each. A panel that is not on screen **MUST NOT** be reachable
   by its number. `0` clearing a priority was given up for this — the letter a todo
   already carries clears it, so the key was one of two spellings for one idea.
9. A panel whose rows do not fit **MUST** show a thumb (`▐`) on its inner right edge,
   and a panel whose rows fit **MUST NOT** show one. The track is not drawn: an empty
   column of track glyphs is a second border. A thumb that is always full says nothing,
   and on a narrow panel its column is a row's worth of text. The thumb is dim, so it
   reads as furniture rather than content, and it never replaces the frame's border —
   it sits inside it, one cell in.
10. A band **MUST** stop one column before the panel's text edge — the column the bar
    lives in — whether or not a bar is up. The bar then needs no background of its own
    and no special case: it is drawn on the panel's ground like any other glyph. That one
    column is what a bar costs a panel, and the right edge of a panel is air.
11. The bar **MUST** be drawn **after** the panel's rows. A band that reaches the bar's
    column would otherwise be painted over the thumb, and the symptom is worse than a
    colour: a thumb with a hole in it wherever a banded row crosses it reads as two bars.
    Rule 10 is what makes this a matter of plain ordering rather than a style to keep in
    step in five places.
12. The thumb's **top MUST be at the track's top when the panel is scrolled to the
    top**, and its **bottom MUST be at the track's bottom at the furthest scroll**. It
    **MUST** grow with the ratio of visible rows to rows, so a long list gets a short
    thumb and a list that nearly fits gets a long one. ratatui's `ScrollbarState` takes
    the *number of scroll positions* — the furthest scroll plus one — not the row
    count: given rows, it puts the thumb's start at `position × track ÷ (rows − 1 +
    visible)` and the thumb stops at two thirds of the track however far the panel
    scrolls. That is the bug this rule exists to keep out.

## Why not the alternative

Keeping one implementation that switches on its source — `todos.rs` and
`collection.rs` are two today — is a separate question from the naming, and it is
the smaller half of this RFC: the panels already share `rows::line_rows`, the gutter
width and the band, and only their source differs. Folding them into one module is
allowed by rule 4 without being required by it, and doing it while the names are
still moving would rename the sink as well as the tap.

The alternative that was rejected outright: **one panel that shows everything**,
todos and prose and file rows together, so there is only ever one "list". It was
rejected because a file's prose is not a place a cursor goes, and a list where some
rows are prose has to explain which rows can be acted on — which is the file view's
job, done worse.

## Consequences

- The files panel's rows are places (files, collections), not todos. Its stats are
  about the file, and its keys (`d`, `n`, `c`) act on the row under the cursor — a
  row whose kind decides what the status bar offers.
- Two panels showing todos means two places a numbering bug can hide, and rule 4 is
  what makes that acceptable: the same rules, tested in both.
- The glossary is now load-bearing: code that names a panel differently from the
  glossary is a bug, not a style choice.
- `the_thumb_is_not_banded` reads the buffer's backgrounds rather than its text: the
  thumb's cell is `Reset` while the banded row beside it is the selection's green, which
  is the only way to prove a colour rule from a screen made of strings. It passes by
  construction now: the band stops before the bar (rule 10), so nothing paints there.
- `no_panel_lets_its_band_paint_over_the_thumb` renders two overflowing panels at once —
  the files list and a code file's view, each with a banded row crossing its bar — and
  asserts every thumb cell is on the panel's background **and that the cells of each
  thumb are contiguous**, because the real symptom is a hole, not a colour: the drawing
  order was wrong in the file view and the thumb vanished on the row the cursor was on.
- The thumb's geometry is the one thing about it a person will notice at a glance, and
  it is arithmetic no test would have caught by accident: `a_thumb_starts_at_the_top_and_stops_at_the_bottom`
  asserts the first inner row at the top of a list and the last inner row at the end of
  it, which is what a wrong `ScrollbarState` fails.
- The thumb costs one cell of width in the panel it appears in, and it appears only
  when the panel is full — so a panel can lose its last column of text at the moment
  its content grows. That is the trade for knowing that there is more to scroll to;
  the alternative, a fixed column of gutter, spends the cell on every panel always.

## Open questions

- Should the thumb be clickable, or should a click in the panel's column jump a page?
  Today the panels are keyboard-first and a mouse does nothing there.

- Should the todos panel take the file view's place in a folder session (so `→` on a
  file row lands in a todos panel, not in the file view), with the file view one
  step further? Today the file view is what a file row opens.
- Should the two implementations become one module now that rule 4 pins what they
  share? Named here so it is not decided twice.
