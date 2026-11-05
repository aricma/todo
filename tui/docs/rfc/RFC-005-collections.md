# RFC-005: Collections — tags and people as places

- **Date:** 2026-10-02
- **Related:** [RFC-001](RFC-001-panels-and-their-names.md), [RFC-006](RFC-006-discovery-and-limits.md)

## Context

The line grammar has two words with meaning in it: `#tag` and `@person`, recognised
anywhere in a todo's body. They are the only grouping this program has — there are no
projects, no labels, no folders. A person who writes `#home` on nine todos across
four files is saying those nine belong together, and the program's job is to make
that sentence answerable: "everything I owe about home", in one place, without
leaving the terminal.

A collection is that place. It is not a filter applied to a list: it is a second
source of todos — the files panel lists it beside the files, and opening it shows its
todos as a **todos panel** (RFC-001).

## Behaviour

1. A collection **MUST** be the set of todos carrying a given `#tag` or `@person`
   across every listed file, and the two kinds **MUST** be kept apart: a tag called
   `sam` and a person called `sam` are two collections.
2. Names **MUST** be compared exactly. `#Home` and `#home` are two collections,
   because a tag is a word a person wrote, and the program does not silently rewrite
   their file to make a list shorter.
3. The files panel **MUST** list files first, then tags, then people; inside a kind,
   **biggest first** (todos, not files), ties broken by name so the rows do not move
   between scans.
4. A collection row **MUST** show its count and its done count, the way a file row
   does — it is the same fact about a place.
5. Opening one (`→`, or `enter` on its row) **MUST** draw its todos as a todos panel
   titled `<tag> todos` / `@<person> todos`, in **file order**: files as the scan
   sorted them, and inside a file the order the todos are written.
6. Items **MUST** be grouped under the file they come from, and each item **MUST**
   carry the line number it has in that file (RFC-002).
7. An item **MUST** be edited through the file it lives in — check, edit, delete,
   priority — with the same one-write-per-action rule as the file's own todos.
8. The index panel **MUST** be absent while a collection is open: it would be the open
   file's headings, and moving in it would walk sections the collection is not made of.
9. Reading a collection **MUST NOT** create or delete one. There is no key for either:
   deleting would mean stripping `#tag` from every line that names it, across files,
   and this program does not rewrite what it was not asked to change.
10. A file that cannot be read **MUST** fail the whole gather with a message. Half a
    collection that looks whole is worse than an error.

## Why not the alternative

**A filtered list** ("show only #home") was rejected: it hides the todos that are not
home while pretending to be about home, and it has no place in the files panel, which
is a list of places to go.

**Sorted by name** was rejected in favour of size. A list of forty tags is read to find
the busy ones; a name is easier to find with `f` than by scrolling, and search ranks
across all files rather than one panel's rows.

**Renaming on collapse** (`#Home` folded into `#home`) was rejected: it makes a
collection's count a lie about a file, and this tool's promise is that the file it
writes is the file a person wrote.

## Consequences

- A typo makes a second collection, visibly, with a count of one. That is the honest
  reading of the file, and the fix belongs in the file.
- Adding a tag to a todo makes it appear in a collection — a second surface a change
  lands on, and a second place a test has to check.
- The gather reads files it did not open, so the scan's entries are the promise that
  keeps it cheap: a file that mentions no collection in the query is never read.

## Open questions

- Should a collection row be hideable, for the typo that will never be fixed?
- Should a tag's case-insensitive twin be offered as a hint ("did you mean `#home`"),
  rather than merged?
