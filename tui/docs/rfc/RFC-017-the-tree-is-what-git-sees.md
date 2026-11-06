# RFC-017: The tree is what git does not ignore

- **Date:** 2026-10-16
- **Related:** [RFC-006](RFC-006-discovery-and-limits.md), [RFC-016](RFC-016-a-comment-todo.md)

## Context

RFC-006 decided which files a walk looks at: markdown named `TODO*`, `TASK*` or `README.md`,
with a list of folders never descended into (`node_modules`, `target`, `tmp`, `.git`) and
four limits — depth, files, lines, and a warning for each one it hit.

That worked because the walk only opened markdown files it already wanted. Comment todos
change the cost: to know whether a file holds a `TODO:` comment, the file has to be
**read**. Every file. In a real checkout that means a vendored folder, a `dist/`, a
generated client, a hundred megabytes of `Cargo.lock` and `.min.js`.

The folders this tool skips are a guess. The team's own answer already exists, is written
down in a file they maintain, and is the one their editor, their builds and their CI all
obey: `.gitignore`. So the walk should not guess — it should ask.

## Behaviour

1. When the path given is inside a git work tree **and** `git` can be run, the file list
   **MUST** be what git reports: `git ls-files -z --cached --others --exclude-standard -- .`
   — what is tracked, plus what is new and not ignored. Ignored paths are never listed and
   **MUST NOT** be read.
2. `.git/info/exclude` and a global `core.excludesFile` apply, because git applies them.
   That is the point of asking rather than re-implementing.
3. When there is no work tree — a plain folder, a tarball, a scratch directory — the walk
   **MUST** fall back to the folder list and limits of RFC-006. The tool **MUST NOT** fail
   because git is missing, nor ask the person to install it.
4. The limits still apply, and **MUST** keep applying after the list comes from git: depth,
   files listed, lines in a file, plus two new ones — bytes read per file, and files read
   per walk. Reaching any of them **MUST** be reported as a warning, never silently.
5. A file the walk reads **MUST** be text: bytes that are not UTF-8, or that hold a NUL
   byte, are skipped. Named on the command line, they are an error instead — the person
   asked for that file.
6. A file is listed only when it holds something: a document with a todo, a finding, or
   nothing but headings; a comment todo file with at least one `TODO:` comment the finding
   rule accepts (RFC-019). The rest of the tree is read and dropped, because a file list is
   a list of places to go, not an inventory.
7. The scan **SHOULD** remain fast enough to run on open without a spinner. Reading every
   text file in a repository of this size takes milliseconds; the limits are what keep a
   pathological tree from taking minutes.

## Why not the alternative

**A hardcoded ignore list, extended with `dist`, `build`, `vendor`, `.venv`, …** It is a
list that is always one entry behind, it disagrees with the team's own answer, and it
cannot express `*.generated.rs` or a nested `.gitignore`.

**Re-implementing `.gitignore` matching in Rust.** It is a real format with negation,
directory-only rules and precedence between three files. Git already does it.

**Only tracked files (`git ls-files --cached`).** A todo written ten minutes ago in a new
file would be invisible until it was committed — the exact moment a person is most likely
to be looking for it. `--others --exclude-standard` is the half that fixes this.

**Not reading non-markdown files at all.** Safe, and useless: the whole point of the pass
is that the todos people actually leave in a repository are in the code.

## Consequences

- Discovery depends on `git` being on `PATH` for the fast path, and on the process being
  spawnable. Two `Command` calls per scan, at open and on a reload, not per keystroke.
- A repository with a huge untracked folder that is *not* ignored is read until the byte
  or file limit stops it — and then says so, in the file panel's own footer.
- A file that is ignored today and un-ignored tomorrow appears after the next scan, which
  is the next open or the next outside change. There is no watcher.
- Tests need a temporary `git init` to prove the ignored case, and must skip quietly where
  git is not installed; the fallback path is covered by every other test, which runs in a
  scratch directory with no repository at all.

## Open questions

- Should the scan remember its file list between runs, or is re-reading on every open the
  right trade for never showing a stale tree? Today: re-read, deliberately.
- What should the byte limit be for a file whose *language* makes one line enormous — a
  minified bundle is one line and a megabyte? The line cap catches it after reading; the
  byte cap stops before.
- Should `--check` list the comment todos it will not check, so a person knows they were
  seen? Today it neither reports nor validates them.
