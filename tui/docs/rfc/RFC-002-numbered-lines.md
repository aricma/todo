# RFC-002: Numbered document lines

- **Date:** 2026-10-02
- **Related:** [RFC-001](RFC-001-panels-and-their-names.md)

## Context

A todo seen on screen is only useful if it can be found again — in the editor, in a
diff, in a review comment, in a message to a colleague. Every panel that shows
document lines therefore shows the line's number in a dim **gutter**. It was true of
the todos panel for a file, and of the todos panel for a collection, and it was false
of the **file view**: the one panel that shows the whole document showed no numbers
at all, so the panel where a person reads a todo in context was the one panel where
they could not say which line it was.

## Behaviour

1. A panel that draws document lines **MUST** number them, in a dim gutter down the
   left, using **the line number the file itself has** — not an ordinal position in
   the list. Finding a todo by its number in an editor **MUST** land on that todo.
2. The file view **MUST** number every line it draws, prose and headings included, so
   the number a todo has there is the number it has everywhere.
3. The gutter **MUST NOT** be part of the row's content: it is never banded, never
   editable, and never copied. A copy of a todo is the line as the file has it.
4. A row that wraps **MUST** wrap in what the gutter leaves, and **MUST** start its
   continuation rows under its body — the gutter's width of air, then the line's own
   indent — so a wrapped line has one straight left edge.
5. The gutter's width **MUST** be the digits of the highest line number the panel
   draws, plus one space. A file of nine lines gets the narrowest gutter there is.
6. The cursor, while a todo is edited in place, **MUST** sit in the text after the
   gutter and after the line's lead — the gutter takes columns, and the cursor's
   arithmetic owns that fact exactly once per panel.
7. A **collection**'s items **MUST** be numbered with the line each item has in the
   file it came from, and a collection **MUST NOT** show a gutter for the collection
   itself: there is no such line.

## Why not the alternative

Numbering by position in the list (`1`, `2`, `3` for the first, second, third todo)
is what a list widget gives for free and was rejected: the number would change when a
todo above it is deleted, and it would not be a number anyone could act on outside
this program. The gutter exists to talk to the file, not to the screen.

Hiding the gutter when the panel is narrow (three digits of gutter in a forty-column
terminal) was rejected for the same reason: a number that appears and disappears
makes the panel jump as the file grows, and the todo's number is the one fact that has
to be stable.

## Consequences

- Every panel that draws lines loses the gutter's width from its text width, so a
  wrapped todo in the todos panel and the same todo in the file view can wrap at
  different columns. That is expected: they are different widths, and rule 4 keeps
  each panel internally consistent.
- A test for each panel has to prove the number matches the file, not the row index —
  a fixture with prose above a todo is the cheapest proof.
- The alternative of a number column that only exists while a "goto line" prompt is
  open would have cost no width at all, and is still open (see below).

## Open questions

- Should `g`/`G` accept a typed line number, so the gutter is a way *into* the
  document and not only a way back to the editor?
