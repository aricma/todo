# Changelog

What changed in `lazytodo`, the terminal client. This package has its own version
and its own tag (`lazytodo-v<version>`), independent of the other packages here.

The newest section is the version a release is cut from, so a release adds one at
the top. There is no Unreleased section: the pipeline refuses to release a version
that is not the newest section here or in `tui/Cargo.toml`.

## 0.1.0

The first release.

`lazytodo` reads the todos a repository already has — the ones in its markdown files
and the `TODO:` comments inside its source — and puts them on one screen to be worked
through. What it is for is maintenance: a monorepo or a large repository collects
hundreds of todo lines that nobody reads, because reading them costs a `grep` and
rewriting one costs an editor and a jump. Here the work is one key away from the
cursor, the way `lazygit` made staging one key away. This client is an homage to it:
the same numbered panels, the same unmodified single-letter actions, pointed at todo
lines instead of diffs.

### Finding the work

- Todo files by the names a repository gives them — `TODOs.md`, `TASKS.md` — and `h`
  when you want every other `*.md` in the tree as well.
- `TODO` comments inside source files, so the ones an agent left behind are in the
  same list as the rest: `TODO`, `FIXME`, `HACK` and `XXX`, per language (RFC-020),
  with `TODO(@alice):` and `TODO(#home):` naming a person or a tag (RFC-021).
- The tree is what git does not ignore (RFC-017), and where a limit stops the walk the
  file is listed and marked rather than quietly left out (RFC-006).

### The screen

- Five panels, four numbers: `0`–`3` jump to the panel whose title carries that
  number. Fixed numbers, not reading order, so a key means the same thing when the
  layout changes. Panel `0` is the one you work in — the todos, a collection, or the
  open file (RFC-001).
- Two lines of keys at the bottom: the ones every panel shares, then the ones of the
  panel you are in, and only those that would do something (RFC-009).
- A file row summarises the file rather than listing it: `5/15` done, then how many
  distinct tags and people it mentions, so a file you have not opened still says what
  is in it.

### Working through them

- Check, uncheck, add, delete, rewrite in place, and move a todo across headings. The
  row itself is the editor, and the file is written once the edit is finished
  (RFC-003, RFC-010).
- Priorities `A`–`D` from the Eisenhower map — which is also an instruction an agent
  can be given: do the `A`s and `B`s, or only what is assigned to `@kevin`.
- Collections: every tag and every person as a place to stand, biggest first, so work
  that piled up implicitly can be walked as if it were a file (RFC-005).
- Search over every todo, with the cursor landing on the answer it found (RFC-007).
- `c` copies whatever the cursor is on — a todo's line, a file's path, a heading's
  text — with no selection mode: move, press, paste into a prompt or a commit message.
  A todo leaves as an OPS document
  (<https://github.com/aricma/open-productivity-standard>) and comes back as lines
  (RFC-014).

### Considerations

- The app keeps no state: every action is a write. History is the session's own trail
  (RFC-008), for undoing what you just did — close the app and it is gone, on purpose.
  There is nothing to save, and nothing to lose.
- A write touches the todos and nothing else: headings, prose, code fences and the
  file's own line endings are left as they were.
- A change made by another program is picked up within a second: the file is re-read,
  not overwritten.
- No config file, no network, no account. The repository is the state, and the rules
  that read a file are the rules that write it back.
