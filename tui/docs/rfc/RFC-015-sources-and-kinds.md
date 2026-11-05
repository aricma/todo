# RFC-015: Three sources, and what each one allows

- **Date:** 2026-10-16
- **Related:** [RFC-005](RFC-005-collections.md), [RFC-009](RFC-009-keys.md), [RFC-016](RFC-016-the-inline-todo.md), [RFC-017](RFC-017-the-tree-is-what-git-sees.md)

## Context

Until now the tool had two ways of holding todos: a **todo document** — markdown whose
`- [ ]` lines are the todos — and a **collection**, which is not a file at all but a
reading of every todo in the tree that carries one `#tag` or one `@person`.

A third one exists in every repository, and it is the one a reader is most often looking
for: the `TODO:` comment in the code. `// TODO: handle the retry`, `# TODO: drop this
once 2.0 ships`. Nobody wants to move those into a markdown file before they can be seen
and edited — by the time a person does that, they have stopped reading them.

Adding them is not free, because their line is **code**. A checkbox can be flipped
without changing what the line *is*; a comment cannot be checked, cannot be moved away
from the function it talks about, and cannot be inserted by a tool that does not know
where in the code the thought belongs. The keys have to know the difference, which means
the difference has to be a thing in the program.

## Behaviour

1. There are **three sources** of a todo, and the program **MUST** name them, because a
   person reading a row is reading one of them: a **todo document**, an **inline file**,
   and a **collection**.
2. A path **MUST** decide the kind: a `.md` file is a todo document, and every other file
   the walk reads is an inline file. There is no way to make a `.md` file an inline one,
   and no way to make a `.rs` file a document.
3. The kind **MUST** decide the key set (RFC-016), the same way in three places at once:
   the key that is pressed, the row the status bar draws, and the keys page. A key that
   cannot act **MUST NOT** be offered, and **SHOULD** say why when pressed, rather than
   doing nothing.
4. A collection's rows **MUST** carry the kind of the file they came from, and the key
   set **MUST** follow the row the cursor is on — a `#tag` collection that mixes an
   inline todo with a document todo offers `x` on one row and not on the next.
5. A session **SHOULD** open on a todo document rather than an inline file, when the tree
   holds both: a person opens this tool on the place their todos were written, not on
   whichever source file sorts first.
6. The kind is a property of the **file**, not of the panel: the same todo shown in the
   file view, in the todos panel, and in a collection allows the same keys.

## Why not the alternative

**One key set for everything, with the impossible keys doing nothing.** This is what the
tool would have grown into by accident, and it is the worst of the options: `x` on a
comment would silently write `// TODO: [x] …` or, worse, edit the code around it. A key
that is offered is a promise that it can act.

**A key set per panel.** Panels are where you *are*, not what the file *is*. The same
todo appears in three panels; making the rules a property of the panel means three places
to change, and a fourth the day a new panel is added.

**A separate program for code todos.** `grep TODO` already exists, and it is what people
use. The reason to fold this in is that the tool can then *edit* them in place with the
same undo, the same collections and the same numbers — a second program could not.

## Consequences

- `Source` (`Document` / `Inline`) is on the parsed file, and the file panel's row carries
  it, so nothing has to guess from the extension twice.
- The bar's second row is no longer a pure function of the pane: `hints::Where` gained an
  `INLINE` bit that a binding can be excluded by, so `x`, `space`, `n`, `K`/`J` and
  `ctrl+↑/↓` are not drawn in an inline file.
- Three refusals are new words on screen: that a comment cannot be checked, cannot be
  added to by the tool, and cannot be moved.
- A test has to prove the two halves agree: a key absent from the bar must also be
  refused when pressed, or the screen and the program disagree about the same file.

## Open questions

- Should the file panel mark the kind of each row? It shows the counts, and a code file
  with one todo reads `0/1` exactly like a document with one todo. Today it does not mark
  it; the path's extension is the only tell.
- Should `n` in an inline file offer a commented-out placeholder at the cursor's line —
  `// TODO: ` written for you, in the comment syntax of that language? It would need to
  know each language's comment, which is a table this tool does not have.
