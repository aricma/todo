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
> headers the sqlite-wasm OPFS VFS needs via Trunk.toml. Pass
> `--no-hot-reload` to freeze the served page at the last good build
> while trunk keeps compiling in the background — e.g. while an agent is
> editing the code, so its work never breaks your open session.

**OPTIONS**

- port
  - flags: --port
  - type: string
  - desc: Port to serve on (default 8080)
- no_hot_reload
  - flags: --no-hot-reload
  - desc: Disable browser hot reload and the error overlay; the page keeps the last good build until you restart the server

```bash
trunk serve ${port:+--port "$port"} ${no_hot_reload:+--no-autoreload --no-error-reporting}
```

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

For the end-to-end smoke suite, keep this running and point it at the app
from `../integration-tests` with
`ARICMA_TODO_APP_URL=http://127.0.0.1:8080 mask test aricma-todo`.
