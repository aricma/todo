# RFCs — the terminal client

One file per behaviour of `aricma-lazytodo`. An RFC here answers three questions in
the order a reader asks them: **what does the program do**, **why that and not
something else**, and **what does that cost**. It is written to be true — not to
sell. The landing page and the marketing copy are read *off* these files later; they
never become a reason to write one differently.

Everything a person reads on screen has a name, and the names are in
[../glossary.md](../glossary.md). An RFC uses those names or it is wrong.

## The rules of this directory

- The directory is the state. A file in `rfc/` is **open**: it may change, and it is
  a proposal as much as a record. There is no `Status:` line, because a line can
  disagree with the directory and the directory is what a reader checks.
- Numbering is per directory and never reused: `RFC-001`, `RFC-002`, …
- When this client ships a version, its RFCs are locked as decisions in `../dr/`
  (`DR-001`, …), taking the next DR number in the order they are locked — an RFC's
  number does not travel with it. A DR is never edited; a later DR supersedes it and
  says so.
- An RFC cites DRs. Citing another open RFC is a debt, named as one where it happens.
  This project has no DRs yet, so today the RFCs stand alone.

## How to write one

| section | what belongs in it |
| --- | --- |
| title | `RFC-NNN: what behaviour this is` — a noun, not a slogan |
| **Date**, **Related** | the day it was written; the RFCs and DRs it argues with |
| **Context** | what a person is trying to do, and what the program does today. No adjectives |
| **Behaviour** | the rules, numbered, `MUST` / `SHOULD` / `MAY`. Written so a test can be named after each one |
| **Why not the alternative** | the design that lost, and the thing that killed it |
| **Consequences** | what gets harder, what a user notices, what a test has to prove |
| **Open questions** | what this RFC does not decide, in the words of the question |

## Index

### Written

| # | RFC | the behaviour in one line |
| --- | --- | --- |
| 001 | [Panels and their names](RFC-001-panels-and-their-names.md) | five panels, one meaning each, and one todos panel for two sources |
| 002 | [Numbered document lines](RFC-002-numbered-lines.md) | every panel that shows document lines numbers them with the file's own line |
| 003 | [Editing a todo where it sits](RFC-003-editing-in-place.md) | the row itself is the field, and the file is written once it is done |
| 004 | [The clipboard and its feedback](RFC-004-clipboard.md) | a copy lands in the clipboard the person can paste from, and the header says so |
| 005 | [Collections](RFC-005-collections.md) | tags and people as places todos live, biggest first, read in file order |
| 006 | [Discovery and limits](RFC-006-discovery-and-limits.md) | which files are candidates, which are listed, and where the walk stops |
| 007 | [Search](RFC-007-search.md) | a query over every todo, best first, that puts the cursor on the answer |
| 008 | [History and undo](RFC-008-history-and-undo.md) | the session's own trail, one entry per act, and when an undo must refuse |
| 009 | [Keys, focus and crossing](RFC-009-keys.md) | every binding, what it does, and why that key — with the unmodified spellings |
| 010 | [The two fields](RFC-010-the-two-fields.md) | a new todo under its row, a new file with offers and refusals |
| 011 | [Where messages go](RFC-011-where-messages-go.md) | success in the header, failures and refusals by the keys, the rest beside their panel |
| 012 | [Chapter clusters](RFC-012-chapter-clusters.md) | a chapter is its heading, its own text, then its todos — and no key splits that block |
| 013 | [A todo is a checkbox](RFC-013-a-todo-is-a-checkbox.md) | a bullet is a shape; a checkbox is a state, and only a state is a todo |
| 014 | [A copied todo is a document](RFC-014-a-copied-todo-is-a-document.md) | a todo leaves as OPS JSON and comes back as lines; paths, headings and tags stay strings |
| 015 | [Three sources, and what each allows](RFC-015-sources-and-kinds.md) | a document, a comment todo file, a collection — and a key set that follows the file's kind |
| 016 | [A comment todo is one line of code](RFC-016-a-comment-todo.md) | one line, the literal `TODO:`, `(A)` after the colon, and no check, add or move |
| 017 | [The tree is what git does not ignore](RFC-017-the-tree-is-what-git-sees.md) | the scan asks git, and reads every text file it is given, within limits |
| 018 | [Reading a comment todo file](RFC-018-reading-a-comment-todo-file.md) | a line above each todo, three below, one `…` for the rest, and nothing folded in a document |
| 019 | [Not every line with a `TODO` in it is a todo](RFC-019-not-every-todo-is-a-todo.md) | the rule is a guess, it guesses narrow, and the table of what it misses is normative |
| 020 | [Four words, and a marker each language uses](RFC-020-four-words-and-a-marker-per-language.md) | comments first, then todos in them; `TODO`/`FIXME`/`HACK`/`XXX`; the marker comes from the file's extension |
| 021 | [Who it is for, and which ticket](RFC-021-who-it-is-for-and-which-ticket.md) | `TODO(alice):` is one of the body's people; `#102`/`#PROJ-404` is a ticket and not a tag |

### Next

| # | the behaviour |
| --- | --- |
| 022 | Block comments: `/* … */` and `<!-- … -->` spanning lines, and what a range does to edit, undo and copy |
| 023 | Staying current: the 0.75 s poll, the reload that forgets history, and the conflict that only a person can settle |
| 024 | The CLI: `check`, `format`, `export`, `import` — the same rules without a screen |

Written after this pass: the doc claimed `r`/`ctrl+r` forced a reload. There is no
such key (`state/tests.rs` asserts `r` is not one), and `lazytodo.md` now says so.
