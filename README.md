# aricma-todo

A todo list that lives in markdown — plus the two programs that read it: one for
pipelines, one for people.

| part | what it is | how you run it |
| --- | --- | --- |
| `lib/` | the markdown todo rules: parse, edit, check, format, OPS export | a dependency |
| `cli/` | `aricma-todo` — check, format, export, import; no screen, no `ratatui` | `cargo install --path cli` |
| `tui/` | `lazytodo` — the terminal client, for reading and editing by hand | `cargo install --path tui` |
| `testkit/` | what the suites share: run a binary, read what it said, drive a use case | a dev-dependency |

Everything it depends on is public, so a clone builds with nothing but a Rust
toolchain: `cargo test`. No credentials.

## The terminal client

`lazytodo` is layered: `lib` owns the todo file (parse, check, write), `disk/`
holds the file operations it has no opinion on (make an empty file without
overwriting, remove one, measure one, list folders), `state/` is the machine the
screen is a function of, `ui/` draws that state, `terminal/` is the loop that
joins them, `text.rs` holds every word the user reads, and `helpers/` holds the
small functions more than one layer needs.

The rules it implements — the line grammar, the keys, the history, the command
line — are in [lazytodo.md](./lazytodo.md).

## Reading

- [lazytodo.md](./lazytodo.md) — the terminal client: line grammar, keys, history, CLI.
- [tui/docs/](./tui/docs/glossary.md) — the terminal client's words
  ([glossary](./tui/docs/glossary.md)) and its
  [RFCs](./tui/docs/rfc/README.md), one per behaviour, with an index.
- [use-cases/](./use-cases/README.md) — the use cases as data: `core/` is the
  document's behaviour on any interface, `cli/` and `tui/` are what only that
  interface can be asked. Each interface declares a driver, and the driver is what
  makes it a test.

`mask` actions are in [maskfile.md](./maskfile.md); every one of them is a plain
`cargo` command you can run without `mask`.

## License

MIT — see [LICENSE](./LICENSE).

The browser app this library once grew out of is not part of this repository.
