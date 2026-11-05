# RFC-010: The two fields

- **Date:** 2026-10-02
- **Related:** [RFC-003](RFC-003-editing-in-place.md), [RFC-011](RFC-011-where-messages-go.md)

## Context

This program creates two kinds of thing — a todo and a todos file — and both start as
text someone has to type. Typing is the one part of a TUI that cannot be avoided, and
it is also the part people judge first: a field that loses a keystroke or wraps
wrongly is remembered longer than any feature.

Both fields are the same component (`state/prompt.rs` for the text, `ui/popover.rs`
for the two frames): a cursor that can sit anywhere in the text, a line that wraps in
what the panel leaves, and `esc`/`enter`. What differs is what the field is *about*:
a new todo floats under the row it will land after, because that row is the answer to
"where does this go"; a new file floats in the middle, because nothing on screen is
the folder it will land in.

## Behaviour

1. A field **MUST** keep the cursor where the person put it: `←`/`→`, `home`/`end`,
   `backspace`, `delete`, and typing at the cursor — not only at the end.
2. A field **MUST** wrap in the panel's inner width, and its cursor **MUST** follow the
   text onto the second and third row. A field that wraps but keeps its cursor on the
   first line looks broken.
3. The new-todo field **MUST** float under the row the todo will land after, and its
   text **MUST** be styled as the todo will read: priority yellow, tags cyan, people
   magenta, as they are typed.
4. A field **MUST** submit with `enter` and cancel with `esc`, and cancelling **MUST**
   leave the document exactly as it was.
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
