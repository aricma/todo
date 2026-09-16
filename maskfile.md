# Aricma Todo Actions

This mask file documents the actions you can run in `aricma-todo/`. Install
`mask` first (see the root repo README for the workflow), and follow the
repo-wide command standard in `../DOCS/maskfile-spec.md`.

## build

> Compile the Tailwind bundle (aricma-ui tokens, app-blue accent) and the
> release wasm app into `tmp/dist/`.

```bash
./build-css.sh
trunk build --release
# Path routing (ADR-0014) needs a fallback for deep links: a request for
# /today must still boot the app. `trunk serve` falls back to index.html on its
# own; hugo (preview and CI) serves 404.html, so ship one. A real host needs the
# same rule — rewrite unknown paths to /index.html with a 200.
cp tmp/dist/index.html tmp/dist/404.html
```

## lint

> Run the static gates CI enforces on the core: rustfmt check, then clippy on
> the wasm32 target the app compiles for.

```bash
cargo fmt --check
cargo clippy --target wasm32-unknown-unknown -- -D warnings
```

## test

> Run the native unit tests for the wasm-free core.

```bash
cargo test --lib
```

## preview

> Serve the app locally (dev server, auto-rebuilds on change). Sends the COOP/COEP
> headers the sqlite-wasm OPFS VFS needs via Trunk.toml.
>
> Build isolation: each preview run compiles into its own per-port dirs
> (`tmp/serve-target-<port>` for cargo, `tmp/serve-dist-<port>` for the
> served site), so concurrent servers — other agents, other ports — never
> clobber each other's builds. The shared `target/` + `tmp/dist/` stay
> reserved for `mask build`. Restart a server to move it to a new port's
> dirs; point `--target-dir` at a custom cargo cache to reuse one across
> ports.

**OPTIONS**

- port
  - flags: --port
  - type: string
  - desc: Port to serve on (default 8080)
- target_dir
  - flags: --target-dir
  - type: string
  - desc: Cargo target directory for this server (default: tmp/serve-target-<port>) — isolate concurrent builds per port, or point at one dir to reuse a warm cache across ports

```bash
port="${port:-8080}"
target_dir="${target_dir:-$MASKFILE_DIR/tmp/serve-target-$port}"
export CARGO_TARGET_DIR="$target_dir"
trunk serve --port "$port" --dist "$MASKFILE_DIR/tmp/serve-dist-$port"
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

## ci

> Run the project's CI gate: lint, unit tests, then the release build.

```bash
set -euo pipefail
$MASK lint
$MASK test
$MASK build
```
