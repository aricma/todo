# RFC-006: Discovery and limits

- **Date:** 2026-10-02
- **Related:** [RFC-001](RFC-001-panels-and-their-names.md), [RFC-005](RFC-005-collections.md)

## Context

The program is pointed at a checkout, and a checkout is unbounded: a vendored
dependency with four thousand markdown files, a generated changelog of two hundred
thousand lines, a folder nobody excluded from git. Reading all of it would eat the
terminal's memory and the person's afternoon, and the failure would look like the
program hanging.

So there are two questions and they are answered separately: **which files are even
candidates** (a name convention), and **which of those are worth a row** (content).
Both answers come from `lib/`, not from the screen, because `aricma-todo --check`
walks the same tree and the two must never disagree about what "the files" means.

## Behaviour

1. A **candidate file** **MUST** be a `.md` file whose name starts with `TODO` or
   `TASK` (any case), or is `README.md`. Directories named `.git`, `node_modules`,
   `target`, `tmp` and every dot-entry **MUST** be skipped.
2. A candidate file **MUST** be listed when it holds at least one todo, at least one
   line that meant to be a todo (so `--check` can find it), or **nothing but headings
   and blank lines** — a file made for the first todo is a place to go, and a list that
   hides it makes the file look absent. A file with prose and no todos **MUST NOT** be
   listed: it is not about todos.
3. The scan **MUST** stop at ten levels below the path it was given, keep at most
   1,000 files and treat a file over 10,000 lines as **listed but not opened**. Each
   of those numbers **MUST** live in `lib/src/limits.rs` and nowhere else.
4. Reaching a limit **MUST** be reported, never silent: a line in the files panel's own
   footer, above the list's bottom, naming the folder or the count.
5. A file over the line limit **MUST** be marked in its row and skipped by search and
   by collection gathering — the program will not show half a document as if it were
   the document.
6. `h` **MUST** widen the scan to every `*.md` in the tree, and back. Widening goes
   through the same scan, so the files panel, the search and the collections widen
   together: one switch, one meaning.
7. An explicit file argument **MUST** always be shown, whatever it holds and whatever
   its name is — the person named it.
8. The header **MUST** name the **resolved** root, not the argument, so two sessions on
   two checkouts can be told apart.
9. A **session opened on a file** — not a folder — **MUST** refuse a file it cannot read
   as one of the three sources: a `.md` file is a todo document wherever it lives, and any
   other file **MUST** hold at least one `TODO:` comment (RFC-016, RFC-019). Otherwise the
   session **MUST** fail with a sentence naming the file and what is missing — *neither a
   markdown todo file nor a file with a TODO comment* — rather than open a screen with an
   empty panel and nothing to type into. Being **shown** in a listing (rule 7) and being
   **opened as a session** are two different promises, and only the second is broken here.

## Why not the alternative

**Every markdown file by default** was rejected: a large repository has a hundred files
that are not todos, and a list that hides the useful ones behind ninety useless ones is
a worse list than one that assumes a convention. `h` exists for the team that never made
that convention, and it is one keystroke away.

**A config file listing the paths** was rejected as a second place to keep the same
promise as the file names: the convention is already written in the tree, and a config
would drift from it silently.

**Reading everything and filtering in memory** was rejected because the limits exist to
bound the *read*, not the display: the 10,000-line file is exactly the case where
parsing it is the cost.

## Consequences

- A file named on the command line is still always *shown* — no rule above is narrowed —
  but a session on one now has a floor: something to read. Naming a source file with no
  notes in it is a mistake worth a sentence, not a screen that looks broken.

- The listed-file rule is shared with `--check`, so a change to it is a change to
  what the CLI lints. One test asserts the three reasons a file can be listed.
- A tree that needs more than 1,000 files of todo is a tree whose todos this tool
  cannot see the end of, and the warning says so rather than lying by omission.
- A heading-only `README.md` is listed. That is the rule's price, and `h` is not the
  way back: it is a rare file.

## Open questions

- Should the files panel offer a way to exclude a folder for this session (a vendored
  tree that keeps coming back)?
- Should the line limit be per file, or total across the scan, now that a 10k-line file
  is the common shape of a generated changelog?
