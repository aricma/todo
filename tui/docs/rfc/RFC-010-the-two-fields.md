# RFC-010: The two fields

- **Date:** 2026-10-02
- **Related:** [RFC-003](RFC-003-editing-in-place.md), [RFC-011](RFC-011-where-messages-go.md)

## Context

This program creates two kinds of thing — a todo and a todos file — and both start as
text someone has to type. Typing is the one part of a TUI that cannot be avoided, and
it is also the part people judge first: a field that loses a keystroke or wraps
wrongly is remembered longer than any feature.

Both fields are the same component (`state/prompt.rs` for the text): a cursor that
can sit anywhere in the text, a line that wraps in what the panel leaves, and
`esc`/`enter`. What differs is where the field *is*: a new todo is typed into the row
it will become — the empty line goes into the file first, the highlight moves onto it,
and the row is the field, exactly as `e` has always edited one — while a new file
floats in the middle, because nothing on screen is the folder it will land in.

## Behaviour

1. A field **MUST** keep the cursor where the person put it: `←`/`→`, `home`/`end`,
   `backspace`, `delete`, and typing at the cursor — not only at the end.
2. A field **MUST** wrap in the panel's inner width, and its cursor **MUST** follow the
   text onto the second and third row. A field that wraps but keeps its cursor on the
   first line looks broken.
3. A new todo **MUST** be prepared, not promised: an empty `- [ ]` goes into the file
   under the highlighted row (or at the end), the cursor moves onto it, and the field
   opens **over the screen** for the words. The row says where the todo lands, the field
   says what it says, and neither is left to be imagined. The waiting row **MUST** be
   drawn with a `…` after its checkbox — the row itself says a title is on its way —
   and that hint **MUST** be the screen's only: the file receives the title, never the
   dots. The field's text **MUST** be styled as the todo will read — priority yellow,
   tags cyan, people magenta.
4. A field **MUST** submit with `enter` and cancel with `esc`. Submitting a new todo
   **MUST** write the typed words into the row that was waiting for them. Cancelling
   **MUST** take that row out and put the cursor back on the row it came from, so the
   document and the highlight are exactly what they were. Nothing is written to disk
   before `enter`, so an add that is cancelled or abandoned by quitting is not a line on
   anyone's file.
4a. The prepared row **MUST** be a task the document already holds: counted in the
   file's own numbering, listed in the view, and parsed like any other line. It is an
   empty todo while it waits, which `--check` reports as `no title` (RFC-013).
4b. A new todo's history entry **MUST** be the line appearing — one `Insert` — so
   undoing "new todo" takes the row out and redoing puts that same line back.
5. A field that knows its own refusal **MUST** say so above its controls and **MUST**
   block `enter`. The new-file field refuses a path with `..` (nothing outside the root)
   and a folder that already holds a `TODOS.md` (nothing is overwritten).
6. The new-file field **MUST** show the whole path it will make — `./` grey, what is
   typed in full, `/TODOs.md` grey — so the fixed part is visible before `enter`.
7. The new-file field **MUST** offer up to five folders one level under what is typed,
   marked with `↑`/`↓`, and typing on **MUST** take the marked folder as the next
   segment: marking and typing is one motion, not two.
8. A new file **MUST** get the one name the scan looks for (`TODOs.md`) and a heading
   naming its folder (`# <folder> todos`), because that is the file a person would have
   written.
9. A field **MUST NOT** be opened while another is open: one modal at a time, and it owns
   the keyboard until it is left.

## Why not the alternative

**A form with labelled rows and a Save button** was rejected: it costs a whole screen for
one line of text, and it loses the row the todo will land after — the only context that
makes "where does this go" answerable.

**A floating field with no row behind it** (what the tool did first) was rejected: it
put two things with emphasis on screen — the band on the row the todo lands *after*, and
a frame over the middle — and the row that would hold the todo existed only once the
field was submitted. The fix is the prepared row, not the removal of the field.

**Typing straight into the prepared row** (no field at all) was built and rejected: a
line that reads `- [ ]` and nothing else looks like a broken todo, not like an invitation,
and nothing on the screen then says that a line is open for the person to fill. The
field is what says "type here"; the row is what says "it will go there".

**Path completion as a separate picker** was rejected: the path is one string, and a
picker would have to be entered and left for every segment. Offers inside the field keep
one motion.

**Trimming the typed text** was rejected for the screen and kept for the file: the file
gets what was typed (minus the trailing spaces a line cannot keep), and the field shows
it as typed. Today the state trims before the view sees it, which is why a typed double
space is invisible until a wrap moves it — a known defect, and the reason rule 1 says
"where the person put it" rather than "at the end".

## Consequences

- The same component carries three behaviours (todo, heading, file path) that differ
  only in their offers, their refusal and their title. That is why the frame is shared
  and the *state* differs (`Prompt`, `Create`).
- The new-file field's `TODOs.md` is not configurable: a fixed name is what makes the
  file findable by the same scan that lists it (RFC-006).
- A refusal has to be derivable from the typed path alone, so it is re-derived on every
  keystroke rather than checked on `enter`.

## Open questions

- Should the new-todo field accept `#tag` and `@name` completion the way the file field
  completes folders? The grammar is the same shape.
- Should the trailing-space defect be fixed by showing untrimmed text and trimming on
  submit, or by trimming between words as they are typed?
