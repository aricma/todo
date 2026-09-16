# aricma-todo

Local-first todo SPA: Rust + Leptos compiled to wasm, data stored in the
user's browser via OPFS (SQLite through `sqlite-wasm`), with Inbox / Today /
Upcoming views, natural-language quick-add, and OPS import/export. No
account, no sync — the data never leaves the machine.

**Status:** MVP in planning — see [MVP.md](./MVP.md) (spec),
[PLAN.md](./PLAN.md) (implementation plan), and
[docs/adr/](./docs/adr/README.md) (decisions).

## Workflow

- [TODOs.md](./TODOs.md) — open work tracking
- [CHANGELOG.md](./CHANGELOG.md) — release history

## Commands

All commands live in the [maskfile](./maskfile.md):

```
mask build              # Tailwind bundle + release wasm build → dist/
mask lint               # rustfmt check + clippy (wasm32)
mask test               # native unit tests (core logic)
mask preview            # dev server with the COOP/COEP headers the OPFS VFS needs
mask ci                 # lint + test + build (the pre-push gate)
```

Preview the production build: `mask build && mask preview dist` (serves `dist/`
with the same headers via hugo.toml's `[server.headers]`). CI runs the unit
gates plus the Playwright smoke suite (integration-tests/, which points at a
running app via `ARICMA_TODO_APP_URL`).

Design: [DOCS/DESIGN.md](../DOCS/DESIGN.md) + `aricma-ui` templates, with the
app-domain blue accent.
