# RFC-003: Editing a todo where it sits

- **Date:** 2026-10-02
- **Related:** [RFC-001](RFC-001-panels-and-their-names.md),
  [RFC-004](RFC-004-clipboard.md)

## Context

The reason this program exists is that a todo list is a file, and reading the file
means scrolling away from it to find the todo you want to change. A separate editor
modal would throw away the context that makes the file worth keeping: the prose
above the todo, the todo after it, the two todos it depends on.

So the row is the field. `e` turns the todo under the cursor into the thing being
typed, with the cursor in its body; `enter` writes the file; `esc` puts the row back.
The same field is used for a new todo (`a`), for a new heading (`n` in the index), and
for a todo inside a collection — a collection's item is edited through the file its
line came from, never in a copy.

## Behaviour

1. `e` **MUST** edit the todo where it sits: the row keeps its number, its lead and
   its position, and only its body becomes editable, with the cursor in it.
2. `enter` **MUST** write the whole file once, as one edit, and `esc` **MUST** leave
   the file untouched. There **MUST NOT** be a state where half an edit is on disk.
3. An edit **MUST** be recorded in the **history panel** as one entry, named for what
   it did (`edit`, `set prio B`, `new todo`) — not for the key that did it.
4. A write **MUST** be anchored to the text the file had when it was read. If the file
   changed under the session, the write **MUST NOT** land; the session says the line
   changed and re-reads the file, and the edit stays in memory until the person says
   to overwrite.
5. The field **MUST** show the todo as the row shows it: priority, tags and assignees
   keep their colours while the text is being typed.
6. `prio`, check/uncheck and move **MUST** be edits of the same kind: one write, one
   history entry, undoable, anchored to the bytes that were read.
7. A refusal — a move with nowhere to go, an undo whose line moved — **MUST** say why
   in words, beside the thing it refused, and **MUST NOT** write anything.

## Why not the alternative

A modal editor (open the todo in a floating field) is easier to draw and was
rejected: it hides the prose that gives a bare todo its meaning, and it puts the todo
out of reach of the keys that act on todos (`↑`/`↓` to the neighbour, `c` to copy the
line, `d` to delete it). The in-place row costs the wrapping and cursor arithmetic
that already exist, and buys back the context.

Writing on every keystroke was rejected for a reason that is not performance: a
person who types `e`, changes their mind and presses `esc` expects the file to be
exactly as it was, and a preview-then-undo history entry would make `esc` a lie.

## Consequences

- Every edit needs the document it was read from, so the file is re-read on write and
  a conflict is a first-class outcome, not an error path.
- The history panel becomes the audit trail of the session, which is why it is a
  panel and not a one-line "undo" note: an entry that cannot be undone (its line
  moved) has to be visible as such.
- `enter` and `esc` mean the same thing in every field, so the keys page can say it
  once.

## Open questions

- Should `enter` on an unchanged body write at all (today it does, and it is a no-op
  entry in history)?
- Should an undo that refuses leave a hint about what moved, or only the line number?
