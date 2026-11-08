# aricma-todo — the pipeline client

The command a pipeline runs over the same todo files: no screen,
no `ratatui`, the same `lib/` rules underneath. It and `lazytodo` are two interfaces
over one core, and nothing else is shared.

Not released yet. There is no tag, no archive and no package, so this is a build from
a clone:

```sh
cargo install --path cli
```

```sh
aricma-todo check  [PATH]      # exit 1 on a broken todo
aricma-todo format [PATH]      # checkbox + priority letter case
aricma-todo export [PATH] [--as json|yaml|jsonl|csv] [--out FILE]
aricma-todo import FILE [--into TODOs.md] [--as FORMAT]
```

`PATH` is a file or a folder and defaults to the current directory. Every verb is a
subcommand, so `aricma-todo` with no verb prints the usage and exits 2 rather than
opening a screen.

Exit codes: `0` done, `1` findings, `2` the command line or the filesystem said no.

`check` reports exactly three things: a line that looks like a todo but is not one, a
todo with no title left, and a priority letter outside A–D. Spacing, trailing
whitespace, `[X]` and unrecognised `#tokens` are not its business.

Warnings about the scan's limits are printed to stderr and the file is skipped, or
left out of the document. The numbers are in `lib/src/limits.rs`.

## OPS

`export` writes one OPS document through the reference `ops-lib`. The title is the
line minus its priority token, the tags and assignees stay inside it because that is
how the line reads, and the parsed extras ride along as `metadata`:

```json
{ "id": "md-1",
  "status": "open",
  "title": "@dave fix you #worst-bug-of-the-century",
  "metadata": { "priority": "A",
                "tags": ["worst-bug-of-the-century"],
                "assignee": "dave" } }
```

A title longer than the standard's 1 KiB limit, or an empty one, is skipped with a
line saying so rather than truncated. `import` writes the title back verbatim as the
body and only prepends a priority that `metadata.priority` carries as a single A–D
letter.
