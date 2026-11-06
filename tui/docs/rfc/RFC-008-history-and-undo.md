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
   it is in: `●` in effect, `○` undone — never a checkbox. A checkbox is a todo's state,
   and a write about a todo is not a todo: the two glyphs would read as the same idea in
   two panels, which is exactly the confusion RFC-012 works to avoid. A filled dot is a
   write that is in the file; a hollow one is a write that was taken back.
   The dot and the dependency lanes are the entry's **gutter**, so the band the cursor
   draws **MUST** start at the label and not at the dot: a highlight over a gutter reads
   as a highlighted line number. It still runs from there to the end of the panel's text
   area, less the column the bar lives in (RFC-001 rule 10), so the row reads as filled
   rather than as marked-up words.
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

11. An entry **MUST** be one row by default: a label too long for the panel is cut with
    `…` at the panel's edge. `h` **MUST** switch between that and wrapping, per session,
    and the bar **MUST** say which way it goes — `h unhide` while the labels are one line,
    `h hide` while they are wrapped. One row is the default because the trail is what the
    panel is for: three long write names push a session's history off the panel, and a
    history you have to scroll to read is a history you stop reading.
12. The undo **MUST** be `space` alone, and `x` **MUST NOT** undo or redo anything: a
    history entry is a write, not a todo, so it has no checkbox for the todo's key to be
    the twin of. The two keys were the same key for a while, and that read as a checkbox
    the panel had borrowed from the other one.

## Why not the alternative

**Wrapped rows only.** It was the first shape, and it is the right shape for *one* entry:
the title of a write is worth reading in full. It is the wrong shape for the panel, where
the reader is scanning for "what did I do" and the answer is the label's first few words.
`h` keeps both: a glance by default, and the full label when a row is the one being
examined.


**A band across the whole row, gutter included.** It was there first, and it made the
cursor a fact about the *panel* rather than about an entry: the dot sat on the same green
as the label, so the two things the panel says — "this write is in effect" and "the cursor
is here" — were drawn in one colour and one band. Marking only the label keeps them
apart, and it matches the todos panel, where the line number stays outside the band.


**The todo's own checkbox on a history row.** It was there first, and it was wrong in a way
that only shows at a glance: an entry reading `[ ] check todo · pay rent` looks like a todo
that is not done, so the panel looked like a second todos panel with the boxes out of sync.
The dot says "state of a write" without borrowing the todo's vocabulary.


**A single undo key** (`u` undoes the last write) was rejected: the panel is what answers
"three writes ago", and a program that writes files has to be able to show its own trail.

**Reversing by re-reading the file's previous bytes** (snapshots of the whole document)
was rejected: a snapshot has no meaning to show in a row, cannot be composed with a later
edit, and at 10,000 lines the memory is the file, once per keystroke.

**A file on disk** (a journal next to the todo file) was rejected: the history is the
session's, and a second file in the person's tree would be a second thing to explain,
clean and git-ignore.

## Consequences

- `the_history_band_is_on_the_label_and_not_on_the_dot` reads the buffer's backgrounds:
  the dot's cell is not the selection's green, the label's is, and the band still reaches
  the last cell of the text area — a band that stopped at the words would read as marked-up
  text, which is the shape row R-1 rejected on the way here.

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
