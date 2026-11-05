# Use cases

One folder per **use case**: a thing a person actually wants to do, written down as
data instead of code. The folder name is the name, and every folder becomes one
test per interface that can drive it — so the suite reads like a list of what the
tool is for:

```
run::core/check-a-todo/tui.txt
run::cli/format/fixes-checkbox-and-letter-case/cli.txt
run::tui/history/undo-a-check/tui.txt
```

## Three scopes, and what belongs where

The cases are BDD features, not tests for one binary: a folder says *what should
happen*, and each interface says *how a person does it there*.

| folder | holds | example |
| --- | --- | --- |
| `core/` | behaviour of the todo document itself, the same on every interface | mark a todo done, add one, edit one, delete one, reorder one |
| `cli/` | what only a command line can be asked | flags, exit codes, report shapes, `--into`, defaults |
| `tui/` | what only a screen can do | panels, the keys page, scrolling, search, collections, single-file mode |

A **core** case is written without a single key name or flag in it, so it survives
gaining an interface. `core/check-a-todo` runs in the TUI today because that is
where the verb exists; the day the CLI grows one, `cli.txt` appears in the folder
and the same `given` → `expect` runs there too, with no change to the case.

Interface quirks do **not** belong in `core/`: `n` opening a field, `ctrl+f`,
`--as json`, the keys page, the scan's limits in a list — those are `cli/` or
`tui/`. If a case cannot be said without naming an interface, it is not core.

## A case folder

| file | required | meaning |
| --- | --- | --- |
| `case.txt` | yes | one line: what this use case is. A sentence for a human, never asserted. |
| `details.txt` | core | the long version: who wants it, the given, the expectation, why, and how it behaves. Never asserted. |
| `given.md` | no | copied into the case's scratch directory as `TODOs.md` |
| `expect.md` | no | the exact content `TODOs.md` must hold afterwards. Absent means "must be unchanged" |
| `cli.txt` | CLI | how a command-line user does it (below) |
| `tui.txt` | TUI | how a screen user does it (below) |
| `stdout.md` | no | exact stdout, for a case that prints a document |
| `fixture-<name>` | no | copied as `<name>` — e.g. `fixture-tasks.json` becomes `tasks.json` |
| `bulk.txt` | no | a number: `TODOs.md` is generated with that many todos, so a case can reach a limit without a huge fixture |

**The driver file is the test entry point.** The runners find cases by
`cli.txt` / `tui.txt`, not by `case.txt`, so a folder with no driver for an
interface simply does not run there — no skip, no false green.

`details.txt` is prose, in four labelled paragraphs (`who`, `given`, `expect`,
`why`, `behaviour`). It is what a person reads before deciding whether a failing
case is a bug or a stale expectation, so it says the intention and not the
implementation: "the box changes and nothing else about the line does", never
"`doc.check` is called".

## `tui.txt`

The steps a person takes, in order, and what the finished screen must show:

```
open: file                        # optional: open TODOs.md itself, not its folder
key: right                        # a named key, or one character
text: ship it #infra @dave        # literal characters; a trailing space is kept
paste: [{"title": "pay rent"}]     # a bracketed paste, as a terminal sends it
wait: esc cancel · enter add      # poll the screen until this appears (5 s)
screen: [x] (A) pay rent #home    # the finished screen must contain this
```

`key:` names: `tab backtab up down left right home end delete space enter esc q
ctrl-c ctrl-f ctrl-u cmd-f alt-up alt-down meta-down shift-down ctrl-down`, or any
single character. The modifier forms exist because which sequence a terminal sends
differs: `cmd-f` is the kitty protocol's encoding of a command key (`CSI 102 ; 9 u`),
and a terminal that does not speak it simply sends nothing. The app itself only needs
plain letters for its own keys, so a case can nearly always use one.

The steps **must end with a `wait:`**: the file is compared as soon as the screen
settles, and the wait is what proves the last action landed before that. `screen:`
lines come after the steps and are checked on the finished screen — the drawn
expectation, which is the only thing a screen user can see.

## `cli.txt`

One argv line, then what the run must print and return:

```
import {dir}/tasks.json --into {input}
exit: 0
stdout: 1 added
stderr: skipped
```

| key | meaning |
| --- | --- |
| (anything else) | argv, split on whitespace |
| `exit: N` | the exit code (default `0`) |
| `stdout: TEXT` | `TEXT` must appear in stdout; repeatable |
| `stderr: TEXT` | `TEXT` must appear in stderr; repeatable |

No quoting: argv is split on whitespace, so a case needing a space in an argument
cannot be written this way — write a real test instead.

## Substitution

In `cli.txt` only:

| placeholder | becomes |
| --- | --- |
| `{input}` | the absolute path of `TODOs.md` in the scratch directory |
| `{dir}` | the scratch directory itself |

## After the run

- argv: `exit`, then every `stdout:` / `stderr:` substring. With none and no
  `stdout.md`, stdout must be empty.
- `stdout.md`, when present, is compared exactly (trailing whitespace ignored).
- `expect.md`, when present, is compared to `TODOs.md` byte for byte.
- with no `expect.md`, `TODOs.md` must be **unchanged** — a case that only reads
  proves it did not write.
- `screen:` lines, for a TUI case, must be on the finished screen.

Every case runs in a fresh temp directory, so `{dir}` and `{input}` are absolute
paths that never collide.

## Adding one

1. Pick the scope: is this about the document (`core/`), the command line
   (`cli/`), or the screen (`tui/`)?
2. Make `case.txt`, `given.md`, `expect.md` — and `details.txt` if it is core.
3. Add `tui.txt` and/or `cli.txt`; that is what makes it run.
4. `cargo test --test cases` in `tui/` or `cli/`, or `cargo test check-a-todo` for
   one case by name.
