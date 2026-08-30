# Aricma Todo Actions

This mask file documents the actions you can run in `aricma-todo/`. Install
`mask` first (see the root repo README for the workflow).

## build

> Compile the Tailwind bundle (aricma-ui tokens, app-blue accent) and the
> release wasm app into `tmp/dist/`.

```bash
./build-css.sh
trunk build --release
```

## test

> Run the native unit tests for the wasm-free core, plus the same
> fmt/clippy gates CI enforces.

```bash
cargo fmt --check
cargo clippy --target wasm32-unknown-unknown -- -D warnings
cargo test --lib
```

## serve

> Start the dev server (auto-rebuilds on change). Sends the COOP/COEP
> headers the sqlite-wasm OPFS VFS needs via Trunk.toml.

**OPTIONS**

- port
  - flags: --port
  - type: string
  - desc: Port to serve on (default 8080)

```bash
trunk serve ${port:+--port "$port"}
```

For the end-to-end smoke suite, keep this running and point it at the app
from `../integration-tests` with
`ARICMA_TODO_APP_URL=http://127.0.0.1:8080 mask test aricma-todo`.

### dist

> Preview the production build: serves `tmp/dist/` with the COOP/COEP headers
> the sqlite-wasm OPFS VFS needs (hugo.toml `[server.headers]`, same ones
> `trunk serve` sends). Used locally and by CI.

**OPTIONS**

- port
  - flags: --port
  - type: string
  - desc: Port to serve on (default 8080)

```bash
hugo server --port ${port:-8080} --bind 127.0.0.1
```
