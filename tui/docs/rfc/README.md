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

### Planned

Not written yet. Each is a behaviour a person can see, so each will get a file.

| # | the behaviour |
| --- | --- |
| 005 | Collections: tags and assignees as places todos live, sorted by size, opened from the files panel |
| 006 | Discovery and limits: which files are listed, ten levels, the line ceiling, `h` for the whole tree |
| 007 | Search: the query, the ranking, and the jump that leaves the cursor where it landed |
| 008 | History and undo: the session's entries, the graph of dependencies, why an undo can refuse |
| 009 | Keys, focus and crossing: `tab`, `←`/`→`, the status bar's spelling of what works here |
| 010 | The two fields: new todo and new todos file, their offers, refusals and wrapping |
| 011 | Where messages go: header, status bar, panel footer, popover — and what is a refusal |
