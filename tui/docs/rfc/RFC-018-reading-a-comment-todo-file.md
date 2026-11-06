# RFC-018: Reading a comment todo file — two lines of code, and a fold

- **Date:** 2026-10-16
- **Related:** [RFC-002](RFC-002-numbered-lines.md), [RFC-012](RFC-012-chapter-clusters.md), [RFC-016](RFC-016-a-comment-todo.md)

## Context

A todo document is read as a document: its prose is context a person wants in full, and
the file view dims it and clips it to one line but never hides it. A comment todo file is not
that. It is a thousand lines of code with a note somewhere in the middle, and the file view
of it would be a thousand dim lines with one bright one — a screen that is almost entirely
noise, and a reader scrolling to find the second todo.

The code around a todo *does* matter: `// TODO: handle retry` means something different
inside a `for` loop than at the top of a file. But only the lines near it matter, and a
codebase's `TODO:` comments are rarely neighbours.

So the view needs a rule for what "around" means, and it has to be a rule a person can
predict without counting.

## Behaviour

1. In a **document**, every line is shown, as it always was. Folding **MUST NOT** apply to
   a todo document: its prose is the document.
2. In a **comment todo file**, a todo's line is shown, and **MUST** be shown with up to two
   lines of code above it and two below it — `CONTEXT = 2` — so the note is read with the
   code it is about.
3. Lines that fall outside every such window **MUST** be replaced by a single `…` row at
   the start of the run they hide, and that row **MUST NOT** be numbered: the gutter is
   the file's, and a fold does not stand at a line of the file.
4. A run of **five or fewer** lines outside the windows **MUST** be shown rather than
   folded. Five is the fold's own cost plus the two lines each side of it — folding it
   would save one row and cost the reader a scroll to see the code behind the dots.
5. Every line shown **MUST** keep the file's own number (RFC-002), including the code
   lines around a todo, and the todo's row is drawn as a todo row: dim lead, coloured body.
6. The todo the cursor is on **MUST** always be in a window: the windows are computed from
   the todos, not from the scroll position, so a fold can never hide the row being edited.
7. The fold **MUST** be a property of the view, never of the file. Nothing is written back;
   the line numbers and the save are of the whole file, and an edit lands on the line the
   gutter says.

## How it is proven

`use-cases/tui/read-a-comment-todo` opens a 24-line Rust file holding two comments: the
screen shows the two lines either side of each, one `…` for the eight lines between them,
and the file panel's row, the dim lead and the keys `e` and `d` — the case fails if the
windows, the fold or the bar change.

## Why not the alternative

**Show only the todo lines, like a grep result.** It reads as a list of matches, not as a
file — and then the view and the todos panel are the same panel twice. The two lines of
code are what make the view worth having.

**Show the whole file, dim.** Honest, and unusable: the ratio of noise to signal in a real
source file makes the second todo a scroll away. The dimming rule is right for prose in a
document, where every line is written to be read.

**Fixed chunks — five lines after each todo, then a fold.** This is the shape the first
sketch had, and it shows the wrong five: the code *below* a todo and none of the code
above it, so a note at the end of a function loses the function. Two above and two below
keeps the note inside its block.

**Collapsing to a count — `… 42 lines`.** It is a number of lines, which tells a reader
nothing about what is in them, and it competes with the todo's own numbers for the same
kind of attention.

## Consequences

- The file view's row arithmetic now has zero-height lines: things that scroll by line
  index (`offset_for`, the cursor position, the "two rows for air" the reader can ask for)
  must all agree that a hidden line takes no row. That is where a bug would hide.
- A file whose todos are close together reads almost as a whole file, which is correct:
  near-neighbours in the code are why the fold has nothing to do.
- `CONTEXT` is one constant in one place. It is a taste decision, not a law; two is what
  keeps a function's signature above a note and its first statement below.
- A test must prove both directions: a long file folds and hides its far lines, and a gap
  of exactly five lines is shown whole.

## Open questions

- Should the fold row say how many lines it hides, and should that be the only place a
  line count appears? Today it is the bare `…`.
- Should a todo at a file's very end or start get more context on the one side it has?
  Today the window is symmetric and simply runs out.
- Should the fold open under the cursor — the row the cursor is on, unfolded, so `↓` walks
  the code and not only the windows? That is a reader mode this version does not have.
