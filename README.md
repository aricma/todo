# aricma-todo

Local-first todo SPA: Rust + Leptos compiled to wasm, with the todos in a
SQLite database (through `sqlite-wasm`) that the app keeps in the browser when
it can (OPFS) and in memory when it cannot, plus Inbox / Today / Upcoming views,
natural-language quick-add, and OPS import/export. That database can also be
mirrored to a remote the user owns — Google Drive, WebDAV, or S3 (ADR-0015,
ADR-0016) — so nothing about the app requires a server of ours.

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
mask preview -b minio   # …and bring up a local backend to try syncing against
mask ci                 # lint + test + build (the pre-push gate)
```

## Local backends

[docker-compose.yml](./docker-compose.yml) runs the remote servers the app can
sync to, from the same configs in [docker/](./docker) that the integration tests
use — so what you click through by hand is what the tests exercise:

```sh
mask preview --backend all          # all four, then serve the app
mask preview -b nginx,seaweedfs     # only some
mask preview backends               # start them and exit (default: all)
docker compose down                 # stop them
```

| backend | paste into the app's storage settings |
|---|---|
| nginx (8081) | WebDAV — `http://127.0.0.1:8081/aricma-todo` |
| apache (8082) | WebDAV — `http://127.0.0.1:8082/aricma-todo` |
| minio (9100, console 9101) | S3 — bucket `aricma-todo`, keys `aricma-test` / `aricma-test-secret` |
| seaweedfs (9200) | S3 — bucket `aricma-todo`, any keys |

The buckets are created for you, and each backend's data is bind-mounted under
`tmp/backends/<backend>/` — so the `todos.db` the app pushed can be inspected on
disk (Docker is the only requirement).

Preview the production build: `mask build && mask preview dist` (serves `dist/`
with the same headers via hugo.toml's `[server.headers]`). CI runs the unit
gates plus the Playwright smoke suite (integration-tests/, which points at a
running app via `ARICMA_TODO_APP_URL`).

Design: [DOCS/DESIGN.md](../DOCS/DESIGN.md) + `aricma-ui` templates, with the
app-domain blue accent.
