# RFC-013: A todo is a checkbox, not a bullet

- **Date:** 2026-10-02
- **Related:** [RFC-012](RFC-012-chapter-clusters.md), [RFC-003](RFC-003-editing-in-place.md), [RFC-009](RFC-009-keys.md)

## Context

Markdown bullets are everywhere, and most of them mean nothing to this program: a
chapter's summary, a changelog, a shopping list, an argument with three points. A bullet
is a **shape**. What makes a todo a todo is a **state** — it is either done or it is
not, and this program's entire job is showing and changing that state: the checkbox is
what `x` writes, what the counts count, what the collections gather, what search finds.

So the grammar is narrow on purpose. A todo is a bullet **with a checkbox**. A line that
opens like a task and carries no usable checkbox is prose: drawn dim, one row, clipped,
never counted, never checkable. And a line that carries a broken checkbox — `[y]`,
`[]`, `[ ]` with no space after it — is not quietly demoted to prose either: it is a
**finding**, which is the lint `aricma-todo --check` reports, because a person meant
something by it and a file is not fixed by being ignored.

The opposite reading — "a bullet is an unchecked todo" — is what a naive parser does, and
it is the one that rewrites people's documents: every summary list would become a list of
open todos, and pressing `x` on one would write `[x]` into a paragraph of prose.

## Behaviour

1. A **todo** **MUST** be a line of the shape: optional indentation, a list marker, a
   space, a checkbox, a space, a non-empty title. A checkbox is `[` + one character +
   `]`, and that character **MUST** be a space (not done), `x` or `X` (done).
2. A line without a checkbox **MUST** be **prose**, whatever it opens with. It **MUST
   NOT** be counted, checked, edited, moved, deleted, searched or gathered into a
   collection — no key in the program can act on it, because no key has a state to act
   on.
3. The program **MUST NOT** convert a bullet into a todo, and **MUST NOT** assume a
   checkbox where there is none: the absence of a state is not an unchecked state.
4. A line that carries a checkbox-shaped bracket but fails the shape **MUST** be
   reported as a finding — `malformed task line` — and **MUST NOT** be silently treated
   as prose. `[y]`, `[]` and a box with no space after it are the cases.
5. A todo with nothing readable left for a title **MUST** be a finding (`no title`):
   `- [ ]` on its own, and a line made only of `#tag`s or `@who`s, are the cases. It
   stays a todo — a checkbox is a state the person can change — and `--check` is where
   it is reported. The empty one has a second job: it is the row a new todo waits in
   (RFC-010), prepared in the file before its title is typed.
6. A write **MUST** keep the marker and the indentation it found. `* [ ] x` stays `*`,
   never becomes `-`; the file's own style is not the program's to normalise.
7. A bare bullet **MUST** be treated as prose **with a bullet**, not as "a todo that is
   missing its state". The tool does not claim to know what the person meant; only a
   hint they accept may change such a line (see below).
8. The canonical marker is `-`. `*`, `+` and ordered markers (`1.`, `1)`) are accepted
   today and a line using them is a real todo. Narrowing that is a parser change, not a
   documentation change — it is an open question below, and it **MUST NOT** be decided
   silently by a hint.

## Why not the alternative

**A bullet is an unchecked todo** was rejected as the naive reading: it turns every
ordinary markdown list into work items, makes `x` write into prose, and inflates every
count and every collection. The cost of the narrow grammar — a file whose todos were
written as `- foo` shows nothing — is paid once, visibly, instead of permanently,
invisibly.

**Auto-converting on save** (`- foo` → `- [ ] foo` whenever the file is opened) was
rejected: the program would rewrite lines nobody asked it to touch, and a sentence that
was a summary would come back as a task.

**A "maybe a todo" row in the panels** was rejected: two kinds of row that look alike and
answer to different keys is a screen nobody can learn. The report belongs to `--check`,
which is a lint run on the file, not a place to work.

## Consequences

- The dividing line is visual and total: **dim prose, bright todos**. A chapter's summary
  reads as text, and stays text.
- A file written entirely in bare bullets is, to this program, a file with no todos. It is
  listed only if it holds something else worth a row (RFC-006), and the honest remedy is
  a hint the person accepts — not a silent rewrite.
- Two reporting paths now exist and must not merge: a **finding** (`--check`, about the
  file's own rules) and a **hint** (a suggestion a person accepts with `Y`). The `(D)`
  todo about auto-fix hints in the headings panel is where the second one gets invented;
  this RFC only says what such a hint may and may not do — it may offer `- [ ] ` for a
  bare bullet, and it may not write it without `Y`.
- `format` fixes the case of the checkbox and of the priority letter, and nothing else;
  the canonical-marker question is not `format`'s to answer.
- A move steps over a bare bullet list as prose (RFC-012's sharpest edge), which is the
  same rule seen from the other side: it is a chapter's introduction, not its cluster.

## Open questions

- Should `*`, `+` and ordered markers keep counting as todos, or should only `-` — with
  the others becoming findings? Today they are todos, and a file that uses them is not
  wrong.
- Should a bare bullet adjacent to a cluster be *reported* (a hint offered), or is the
  absence of a hint the absence of a claim?
- When a hint is accepted and `- foo` becomes `- [ ] foo`, what does the history entry
  say (RFC-008)? `new todo` would be a lie — nothing was typed; `fixed bullet`?
- Does `- [ ]` in a **fenced** code block count? It does not today: a fenced line is
  never a todo, and the finding path skips it too. That is deliberate: an example is not
  a task.
