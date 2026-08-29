# Aricma Todo Actions

This mask file documents the actions you can run in `aricma-todo/`. Install
`mask` first (see the root repo README for the workflow).

## build

> Compile the Tailwind bundle (aricma-ui tokens, app-blue accent) and the
> release wasm app into `dist/`.

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

```bash
trunk serve
```

For the end-to-end smoke suite, keep this running and point it at the app
from `../integration-tests`:

```sh
ARICMA_TODO_APP_URL=http://127.0.0.1:8080 mask test aricma-todo
```

To preview the production build instead, serve `dist/` with the same headers
(`node serve.mjs`, see README).
