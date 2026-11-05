# RFC-014: A copied todo is a document, and a paste writes lines

- **Date:** 2026-10-02
- **Related:** [RFC-004](RFC-004-clipboard.md), [RFC-013](RFC-013-a-todo-is-a-checkbox.md), [RFC-008](RFC-008-history-and-undo.md)

## Context

`c` was built to pass a todo from one file to another. It copied the file's line —
`- [ ] (A) pay rent #home @sam` — and pasting it into another todos file worked, because
a line is a line. Everything else about that copy was a guess: the priority is a
*positional* token, so a line that loses a space or gains an indent pastes as a
different todo; a line copied out of a document that is not this tool's markdown is not
a todo at all; and the tags and people are words inside a sentence, so nothing but a
human reading the line can tell them apart.

Meanwhile the same program already has a format for exactly this: the Open Productivity
Standard, which the CLI exports and imports, and which says a task is a title, a status,
and metadata — not a string with conventions in it. A copied todo can be that document.
It survives the round trip with its structure intact, it can be pasted into anything
that speaks OPS, and an OPS document from somewhere else — a `--as json` export, another
tool, an agent that was handed a todo to work on — can be pasted here without a
translation step. An agent does not care which of the two forms it is handed, and a
person should not have to.

So: a todo leaves as a document and comes back as lines. The person pasting never sees
the JSON — the file gets markdown, which is the only thing this program ever writes.

## Behaviour

1. `c` in a todo panel, the file view or a collection **MUST** put the selected todo on
   the clipboard as an **OPS document** of one task: `title` (the body without the
   positional priority token), `status` (`open`/`done`), and `metadata` with
   `priority`, `tags` and `assignee`/`assignees` when the todo carries them.
2. A file's path, a heading's text and a collection's word **MUST** stay plain strings.
   They are strings, a person uses them as strings, and nothing about them is lost by
   being text.
3. The success message **MUST** name the todo by its **line**, not by the document:
   `copied - [ ] (A) pay rent #home @sam`. The file's line is what the person recognises;
   the document is what the clipboard holds.
4. A todo with no title (an empty row being typed, or a line of tags only) **MUST**
   refuse to copy, saying why. It **MUST NOT** quietly fall back to the markdown line:
   a paste that receives plain text does not know what to do with it, and a silent
   change of format is worse than a refusal.
5. Terminals **MUST** be put in **bracketed paste** mode for the session, and taken out
   of it on the way out and from the panic hook — a paste that arrives as a hundred key
   presses is a hundred keys, some of them `d` and `x`.
6. A paste **MUST** be one event, not keystrokes, and what it means depends on where it
   lands:
   - a field is a text field, so a paste is **text**, inserted at the cursor, control
     characters dropped;
   - a todo panel or the file view reads it as an **OPS JSON document** and writes its
     tasks as todo lines, depth first, directly under the highlighted todo (at the end
     of the file when nothing is selected);
   - anywhere else **MUST** refuse and say where a pasted todo goes.
7. The file **MUST** receive todo lines and nothing else: the document's own bytes are
   never written into it. A paste of three todos is three lines with their `(A)` tokens,
   `#tags`, `@people` and checkboxes as this file writes them.
8. A paste that is not an OPS document **MUST** refuse and write nothing — the message
   says so and names what a paste is for. A document with no tasks in it **MUST** say
   that it holds none.
9. A paste **MUST** be one history entry per line, newest last, so undo peels it from
   the bottom up. The history is per line (RFC-008); a paste of five todos is five lines
   appearing, and being able to take them back one at a time is what makes pasting into
   a file someone cares about safe.
10. A paste **MUST** be saved like any other write, and **MUST** say how many todos
    arrived (`pasted 2 todos`).

## Why not the alternative

**Keep copying the markdown line** was the first implementation, and it is what a person
gets from any editor. It loses exactly the thing this tool knows: which words are tags,
who is assigned, whether the priority token survived, whether the box was a box. It also
cannot receive a document from anywhere else.

**Copy JSON but paste it as text** (paste the document into the file, let the person
clean it up) was rejected: it writes a format the file was never in, and the next
`--check` would report it as prose.

**Accept pasted markdown too** — recognised as `- [ ] …` lines — was rejected for now,
not forever: it is the one thing on the clipboard this program could read but chooses
not to trust, because a line pasted from an editor is exactly the guess this RFC is
avoiding. It is an open question below, not a closed door.

**One history entry for the whole paste** was rejected: an entry is one line's edit, and
a five-line entry would need an undo that no other key has. Peeling is cheap, honest,
and the message already said how many lines arrived.

## Consequences

- The clipboard holds JSON for a todo. Pasting into a chat, an editor or an issue tracker
  shows the document — that is the cost, and it is the same document an export writes, so
  it is at least a format those places can learn.
- Copy and `--as json` share one builder (`ops::one` and `ops::write` both go through the
  same per-task function), so the two cannot disagree about what a todo is.
- Bracketed paste is a terminal mode; a terminal that ignores it sends the text as keys,
  which is what happened before. Over ssh, paste is the remote terminal's business, like
  the clipboard itself (RFC-004).
- The paste path reuses the **import** half of the CLI's OPS support, so a document of
  nested tasks lands as indented todos and a `done` task lands checked.
- A paste does not move the cursor to the end of what arrived; it moves it to the first
  pasted todo, because that is the one the person is about to look at.

## Open questions

- Should a paste of plain `- [ ] …` lines be accepted as-is, or is refusing it the
  honest answer?
- Should a paste of **YAML** (the other format the CLI writes) be read as well? Today it
  is a bookkeeping-free "no", because `c` writes JSON.
- Should a copy of several todos ever exist — a range, a selection — and would that be a
  document with several tasks, or several pastes?
- A todo's indented children are flattened on export today (the markdown document does
  not nest tasks in the model). Copying one would either lose its children or need the
  tree to be read: which?
