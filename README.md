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

```
cargo test                          # native unit tests (core logic)
./build-css.sh                      # compile Tailwind (aricma-ui tokens, app accent) → public/styles.css
trunk serve                         # dev server (http://localhost:8080)
trunk build --release               # static SPA build → dist/
```

CI (`aricma-todo` job): fmt check, clippy, tests, release build, and the
Playwright smoke suite (integration-tests/).

Design: [DOCS/DESIGN.md](../DOCS/DESIGN.md) + `aricma-ui` templates, with the
app-domain blue accent.
