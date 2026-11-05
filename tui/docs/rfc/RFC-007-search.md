# RFC-007: Search

- **Date:** 2026-10-02
- **Related:** [RFC-005](RFC-005-collections.md), [RFC-008](RFC-008-history-and-undo.md)

## Context

A tree holds a few hundred todos, and the one a person wants is rarely the one under
the cursor. There are two ways to find it: narrow the list (a filter) or search
everything (a query with results). The first leaves them looking at a list that has
quietly stopped being the list; the second can put the cursor on the answer, which is
what they actually wanted.

Search is a **popover**: it floats, it owns the keyboard while it is up, and it is
about the whole scan rather than about the panel behind it.

## Behaviour

1. `f` **MUST** open the search popover from any panel. `esc` closes it and leaves the
   cursor where it was.
2. The query **MUST** survive closing: opening it again is one keystroke, not retyping
   the last search.
3. The search **MUST** cover every listed file's **todos** — not prose, not headings —
   because a hit is something to jump to and act on.
4. Matching **MUST** be fuzzy over the todo's body, ranked best first; equal scores
   **MUST** fall back to path and line so the list is stable between keystrokes.
5. A hit **MUST** show the file (as the panel displays it), the line number and the
   todo's body, because two todos with the same text exist and the line is what tells
   them apart.
6. `enter` **MUST** jump to that todo and leave the cursor on it, in the panel that
   owns it — the file view for a file's todo, the collection's todos panel for a
   collection's — never in a third place.
7. An empty result **MUST** be information, not a failure: `no todo matches`, in the
   popover, not in the error path.
8. A file that cannot be read **MUST** say why under the query, and the rest of the
   results **MUST** stay usable. A file over the line limit **MUST** be skipped
   silently — it was never read.
9. `ctrl+u` **MUST** empty the query. `x` **MUST NOT**: this is a text field, and a key
   that erases what was typed makes searching for `xfce` impossible.
10. The popover **MUST** offer `enter` in its own border only when there is a hit, and
    the status bar **MUST** agree with that border, key for key.

## Why not the alternative

**Filtering the current list** was rejected: it answers "which of these" when the
question is "where is it", and it cannot cross files, which is where the answer usually
is.

**Searching prose and headings too** was rejected: those rows are not actionable — the
cursor cannot sit on a prose line — so a hit list that contains them would offer
results that cannot be jumped to.

**A separate search screen** was rejected in favour of the popover: the search is a
detour inside one task, and a screen would cost the panel behind it, which is where the
person goes back to.

## Consequences

- Every keystroke re-reads the files it searches unless the scan's entries are used;
  that is why the files are asked for rather than kept (a kept copy is one more thing to
  remember to update when a file changes on disk).
- Ranking is not relevance in the search-engine sense; it is "did the letters appear in
  order, and how tightly". Long bodies score worse, which is usually right.
- `f` shadows nothing, because `cmd+f` is taken by the terminal and `ctrl+f` scrolls in
  some terminals (todo: verify which; the binding is `f` alone today).

## Open questions

- Should the query be able to name a scope (`#home fix` — a collection within the
  search), or is that two features pretending to be one?
- Should a hit list remember its selection when the query is reopened?
