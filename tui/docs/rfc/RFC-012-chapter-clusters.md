# RFC-012: Todos cluster at the end of a chapter

- **Date:** 2026-10-02
- **Related:** [RFC-001](RFC-001-panels-and-their-names.md), [RFC-008](RFC-008-history-and-undo.md), [RFC-009](RFC-009-keys.md)

## Context

A todos file is written in chapters: a heading, then whatever that chapter has to say —
prose, a table, a screenshot, a code block — and then its todos. The todos of a chapter
are written together, as a block, at the bottom: a list a person reads top to bottom
once they know what the chapter is about, and the place the next todo goes.

That is not a rule this program invented or can enforce; it is what everyone writes,
because a todo wedged between a heading and the paragraph explaining it is a todo in the
wrong place. But the program *acts* on documents, so where it puts a line is a promise
about that layout, and every key has to agree about it: a move that dropped a todo
between a heading and its introduction would split the block it belongs to, and the
file would stop looking like the files people write.

So the layout gets a name and a rule, and the rule is what the keys are held to.

## The shape

```
## the chapter
its own text: paragraphs, tables, images, code — the introduction
- [ ] first todo
- [ ] second todo        ← the cluster: contiguous, at the end, in written order
```

1. A **chapter** is a heading and everything under it, up to the next heading of any
   level or the end of the file.
2. A chapter's **introduction** is the text between its heading and the chapter's first
   todo.
3. A chapter's **cluster** is its todos, in file order. The program **MUST** treat a
   cluster as one block: a key that changes the file **MUST NOT** land a todo in the
   middle of one, and **MUST NOT** leave a todo on the far side of the introduction
   from the todos it joined.
4. A chapter with no todos **MUST** be treated as having an empty cluster at the end of
   its introduction — that is where the first todo goes.
5. The program **MUST NOT** enforce the shape: a cluster split by a paragraph is legal
   markdown, the file is the person's, and no key rewrites a line nobody asked it to
   change. Clustering is a rule about where the *program* puts things.

## What each key does with it

| key | where the todo ends up |
| --- | --- |
| `a` (add) | under the highlighted row — inside the cluster, because that is where the cursor is |
| `K`/`J` inside a chapter | swapped with the nearest todo that way: the cluster's order is the file's order |
| `K`/`J` across a heading | the far chapter's cluster: **after its last todo** going up, **before its first** going down; the introduction is stepped over with the heading |
| `K`/`J` into a chapter with no todos | the end of that chapter's introduction — the todo starts the cluster |
| `e`, `x`, `A`–`D` | in place: a change of text, state or priority never moves a line |
| `d` (delete) | the line goes; the cluster closes up |
| a Check | in place. `Done ✅` is a chapter a person moves a finished todo to, not a place the program files it |

Two consequences worth naming:

- **One heading per press.** A move crosses the heading it meets and joins the chapter
  behind it. It does not hop again to find a bigger cluster, because a key that moves a
  todo an unpredictable number of chapters is a key nobody can aim.
- **The document's first line is a floor.** The heading above may be the file's own
  title, with nothing above it; there is no chapter to join, so the move refuses and
  says `nothing above it to swap with` rather than filing a todo above the title.

## Why not the alternative

**Insert the todo directly under the heading** (before the introduction) was the first
implementation and is what the RFC replaces: it splits the chapter's own text from its
todos, and the next person to read the chapter reads a list that starts before its
explanation.

**Skip the introduction and the heading as separate steps** (two presses: one over the
heading, one over the text) was rejected: it makes a read-through of the file depend on
knowing which lines are "furniture", and the first press would leave the todo in the one
place the rule forbids — between the heading and its introduction.

**Sort todos to the top of each chapter** was rejected outright: file order is the only
order this program has (RFC-005), and reordering a person's file on open is not a
feature.

## Consequences

- A crossing is one `Inverse::Move` — one lift, one landing — so it is one history entry
  and one undo (RFC-008), and the line numbers of the clipboard copy (RFC-002) still
  mean what they meant.
- The rule is *placement*, so it needs no state: the landing is derived from the lines
  around the heading, and a file that ignores the shape still behaves predictably (the
  nearest todo wins).
- A file written in the other style — a todo at the top of each chapter — will have
  moves that jump to the chapter's end on entry, which is the honest cost of a
  convention being assumed.

## Open questions

- Should a cluster broken by a paragraph be reported as a **finding** (the `--check`
  path already knows how to say "this line meant to be a todo"), and fixed only when a
  person accepts it?
- Should `x` on the last todo of a chapter offer to move it to a `Done ✅` chapter, or
  is that the `d`-and-retype story?
- Is the introduction ever a list that is *not* todos (a bulleted summary)? Those lines
  are prose to this program, so a move would step over them; that is deliberate, but it
  is the sharpest edge of the rule.
