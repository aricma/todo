# aricma-todo

A todo list that lives in markdown and other places. Four surfaces over the same
idea:

| part | what it is | how you run it |
| --- | --- | --- |
| `web/` | the local-first SPA: Leptos wasm, SQLite on OPFS, OPS data model | `cd web && mask preview` |
| `lib/` | the markdown todo rules: parse, edit, check, format, OPS export | a dependency |
| `cli/` | `aricma-todo` — check, format, export, import; no screen, no `ratatui` | `cargo install --path cli` |
| `tui/` | `aricma-lazytodo` — the terminal client, for reading and editing by hand | `cargo install --path tui` |

The terminal client is layered: `lib` owns the todo file (parse, check, write),
`disk/` holds the file operations it has no opinion on (make an empty file without
overwriting, remove one, measure one, list folders), `state/` is the machine the
screen is a function of, `ui/` draws that state, `terminal/` is the loop that joins
them, `text.rs` holds every word the user reads, and `helpers/` holds the small
functions more than one layer needs.

`web/` is its own cargo workspace (wasm-only, built by trunk); `lib/`, `cli/` and
`tui/` share the workspace at this level. `mask` actions are here in
[maskfile.md](./maskfile.md) and the web app's own are in
[web/maskfile.md](./web/maskfile.md).

## Reading

- [lazytodo.md](./lazytodo.md) — the terminal client: line grammar, keys, history, CLI.
- [web/README.md](./web/README.md) — the SPA, and its [docs/adr](./web/docs/adr/README.md).
- [use-cases/](./use-cases/README.md) — the use cases as data: `core/` is the
  document's behaviour on any interface, `cli/` and `tui/` are what only that
  interface can be asked. Each interface declares a driver, and the driver is what
  makes it a test.
- [TODOs.md](./TODOs.md) — what is left, for both the SPA and the clients.
