# Use cases

One folder per **use case**: a thing a person or a pipeline actually does with the
tool, written down as data instead of code. The folder name is the name of the
use case, and it shows up in the test output — every folder here becomes one test,
so the suite reads like a list of what the tool is for:

```
run::check/reports-a-malformed-line/case.txt
run::edit/add-a-todo/case.txt
run::history/undo-a-check/case.txt
```

Adding a use case means adding a folder, not writing Rust. `cli/tests/cases.rs` turns every
`use-cases/cli/**/case.txt` into a test, `tui/tests/cases.rs` every
`use-cases/tui/**/case.txt`, so `cargo test` lists them by path and `cargo test
cli/check` runs one group.

Both runners share the fixture conventions below; only the driver differs. The
parser lives in `testkit/src/cases.rs` (the repo-root `testkit` crate), so a
third project can read the same folders without copying code.

## A case folder

| file | required | meaning |
| --- | --- | --- |
| `case.txt` | yes | one line: what this case proves. A comment for a human, never asserted. |
| `command.txt` | CLI | **one line of argv** |
| `keys.txt` | TUI | steps, one per line |
| `single.txt` | no | TUI: open `TODOs.md` itself instead of the folder (single-file mode) |
| `expectation.txt` | no | what the run must print and return |
| `input.md` | no | copied to the case's scratch directory as `TODOs.md` |
| `fixture-<name>` | no | copied as `<name>` — extra input, e.g. `fixture-tasks.json` becomes `tasks.json` |
| `output.md` | no | the exact content `TODOs.md` must hold afterwards |
| `stdout.md` | no | the exact stdout, for a case that prints a document |

Every case runs in a fresh temp directory, so `{dir}` and `{input}` are absolute
paths that never collide.

## `expectation.txt`

What the run must print and return; `#` starts a comment. A case with no
`expectation.txt` expects exit `0` and empty stdout.

```
exit: 1
stdout: :2: malformed task line
stderr: skipped
```

| key | meaning |
| --- | --- |
| `exit: N` | the exit code (default `0`) |
| `stdout: TEXT` | `TEXT` must appear in stdout; several allowed |
| `stderr: TEXT` | `TEXT` must appear in stderr; several allowed |

The TUI runner ignores `exit` (the session is ended by the test, not by the app)
and checks only the file.

`keys.txt` takes three verbs, in order, and **must end with a `wait:`** — the
screen can only show the state the app reaches after it has written, so the wait
is what proves the write happened before the file is compared:

```
wait: pay rent      # poll the screen until this text appears (5 s deadline)
key: tab            # tab backtab up down left right home end delete space enter esc q ctrl-c, or one character
text: big           # literal characters typed at the cursor; trailing spaces are kept, so `text: big ` types the space
```

`wait:` is the auto-wait: a case never sleeps, it waits for the screen to say
something. Nothing is asserted about the screen itself — a TUI case asserts the
file, which is the observable a user would check.

## Substitution

In `command.txt` only:

| placeholder | becomes |
| --- | --- |
| `{input}` | the absolute path of `TODOs.md` in the scratch directory |
| `{dir}` | the scratch directory itself |

No quoting: argv is split on whitespace, so a case that needs a space in an
argument cannot be written this way (write a real test instead).

## After the run

- `exit` must match.
- every `# stdout:` / `# stderr:` substring must be present. With no `# stdout:`
  directive and no `stdout.md`, stdout must be empty.
- `stdout.md`, when present, is compared exactly (trailing whitespace ignored).
- `output.md`, when present, is compared to `TODOs.md` byte for byte.
- with no `output.md`, `TODOs.md` must be **unchanged** — a case that only reads
  proves it did not write.
