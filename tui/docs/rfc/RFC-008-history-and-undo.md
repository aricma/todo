# RFC-008: History and undo

- **Date:** 2026-10-02
- **Related:** [RFC-003](RFC-003-editing-in-place.md), [RFC-011](RFC-011-where-messages-go.md)

## Context

The program writes a person's file. Every key that changes it — check, edit, priority,
move, delete, add — can be wrong, and a file is not an app: there is no autosave to fall
back to and `git checkout` is a different tool with its own cost.

So the session keeps its **own** history: what this program did to the open file, in
order, undoable one entry at a time. It is a panel, not a one-line "undo" note, because
the interesting question is never "undo the last thing" but "undo the priority change
three writes ago, and everything that hung on it".

## Behaviour

1. History **MUST** be per file and per session. It is not read from git, not written to
   disk, and it **MUST NOT** survive a reload of the file.
2. One user action **MUST** be one entry, named for what it did (`check · pay rent`,
   `set prio B · ship it`, `delete · old idea`), never for the key that did it.
3. The panel **MUST** list entries newest first, and each entry **MUST** show the state
   it is in: `[x]` in effect, `[ ]` undone.
4. `x` **MUST** act on the selected entry — undo an entry in effect, redo an undone one
   — one direction per press, and never past the file it was recorded against.
5. An entry that added or deleted a line **MUST** replay literally: undoing a delete puts
   that line back where it was, redoing it deletes that line again, never a second copy.
6. A write **MUST** depend on the nearest older write to the same line that it undoes —
   not simply whatever came before it. Undoing a write **MUST** undo the writes that
   depend on it.
7. An entry whose dependency is undone **MUST NOT** be redone, and the reason **MUST** be
   shown on that row, derived and only while the cursor is on it — like a field that is
   only validated while it has focus.
8. A write that cannot be undone because its line moved **MUST** refuse and say so; it
   **MUST NOT** be skipped in silence.
9. `d` on an entry **MUST** forget it: forget is not undo. It is how a person says "this
   write is not part of what I might take back".
10. The reload of an externally changed file **MUST** forget the history, and **MUST** say
    it did: every entry is anchored to text that just changed.

## Why not the alternative

**A single undo key** (`u` undoes the last write) was rejected: the panel is what answers
"three writes ago", and a program that writes files has to be able to show its own trail.

**Reversing by re-reading the file's previous bytes** (snapshots of the whole document)
was rejected: a snapshot has no meaning to show in a row, cannot be composed with a later
edit, and at 10,000 lines the memory is the file, once per keystroke.

**A file on disk** (a journal next to the todo file) was rejected: the history is the
session's, and a second file in the person's tree would be a second thing to explain,
clean and git-ignore.

## Consequences

- Undo depends on line identity, so any write this program did not make — an editor, a
  formatter, `git checkout` — invalidates it: the gap between the two is the refusal.
- Two paths can be undone at once only in the order they were written; the dependency
  rule is what keeps that from producing a file that never existed.
- The dependency graph needs its own vocabulary on screen (`╭ │ ╰ ┼`), which is a drawing
  problem this RFC does not own: see `lazytodo.md` for the chain.
- `lazytodo.md`'s key table and the entry names are the two places a rename has to land.

## Open questions

- Should forgetting be undoable (a forgotten entry that the session could bring back)?
- Should a conflict's resolution (keep mine / take theirs, RFC-012) be an entry, so the
  choice itself can be taken back?
