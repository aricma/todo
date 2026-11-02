# aricma-todo

A todo list that lives in markdown and other places. Four surfaces over the same
idea:

| part | what it is | how you run it |
| --- | --- | --- |
| `web/` | the local-first SPA: Leptos wasm, SQLite on OPFS, OPS data model | `cd web && mask preview` |
| `lib/` | the markdown todo rules: parse, edit, check, format, OPS export | a dependency |
| `cli/` | `aricma-todo` — check, format, export, import; no screen, no `ratatui` | `cargo install --path cli` |
| `tui/` | `aricma-lazytodo` — the terminal client, for reading and editing by hand | `cargo install --path tui` |

`web/` is its own cargo workspace (wasm-only, built by trunk); `lib/`, `cli/` and
`tui/` share the workspace at this level. `mask` actions are here in
[maskfile.md](./maskfile.md) and the web app's own are in
[web/maskfile.md](./web/maskfile.md).

## Reading

- [lazytodo.md](./lazytodo.md) — the terminal client: line grammar, keys, history, CLI.
- [web/README.md](./web/README.md) — the SPA, and its [docs/adr](./web/docs/adr/README.md).
- [use-cases/](./use-cases/README.md) — the CLI and TUI cases, run as tests.
- [TODOs.md](./TODOs.md) — what is left, for both the SPA and the clients.
