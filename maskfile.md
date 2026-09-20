# Aricma Todo Actions

This mask file documents the actions you can run in `aricma-todo/`. Install
`mask` first (see the root repo README for the workflow), and follow the
repo-wide command standard in `../DOCS/maskfile-spec.md`.

## build

> Compile the aricma-ui Tailwind bundle (app-blue accent) and the release wasm
> app into `tmp/dist/`. The CSS step is not called here: `trunk build` runs
> `build-css.sh` as its `pre_build` hook (Trunk.toml), so dev server and CI
> cannot drift on a stale stylesheet.

```bash
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
> `--backend` also brings up the local backend servers from `docker-compose.yml`
> (docker required), so a remote can be tried out by hand:
>
> ```sh
> mask preview --backend all          # every backend, then serve
> mask preview -b minio               # just one
> mask preview -b nginx,seaweedfs     # a few
> mask preview backends               # start them and exit (default: all)
> mask preview backends apache        # …or just one
> docker compose down                 # stop them
> ```
>
> Where they answer, and what to paste into the app's storage settings:
>
> | backend | app setting |
> |---|---|
> | nginx | WebDAV — URL `http://127.0.0.1:8081` (the app uses its `aricma-todo/` folder) |
> | apache | WebDAV — URL `http://127.0.0.1:8082` (the app uses its `aricma-todo/` folder) |
> | minio | S3 — endpoint `http://127.0.0.1:9100`, bucket `my-bucket`, prefix blank, keys `aricma-test` / `aricma-test-secret` (console on 9101); the app writes `my-bucket/aricma-todo/todos.db` |
> | seaweedfs | S3 — endpoint `http://127.0.0.1:9200`, bucket `my-bucket`, prefix blank, any keys; the app writes `my-bucket/aricma-todo/todos.db` |
>
> The buckets are created for you, and the data is bind-mounted under
> `tmp/backends/<backend>/`, so the `todos.db` the app syncs can be inspected on
> disk. Ports are overridable (`NGINX_PORT=8090 mask preview -b nginx`).
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
- backend
  - flags: -b, --backend
  - type: string
  - desc: Local backend server(s) to start first — nginx, apache, minio, seaweedfs, comma-separated, or all (see the table above). Default: none, so a plain `mask preview` needs no Docker.

```bash
# The backends, if any were asked for: `preview backends` is the same work
# without a server in front of it.
if [[ -n "${backend:-}" ]]; then
  $MASK preview backends "$backend"
fi

port="${port:-8080}"
target_dir="${target_dir:-$MASKFILE_DIR/tmp/serve-target-$port}"
export CARGO_TARGET_DIR="$target_dir"
trunk serve --port "$port" --dist "$MASKFILE_DIR/tmp/serve-dist-$port"
```

### backends [selection]

> Bring up the local backend servers from `docker-compose.yml` and exit (docker
> required) — the same ones `preview --backend` starts, so they can also be left
> running while something else serves the app. `selection` is one name, several
> comma-separated, or `all` (the default). `docker compose down` stops them.

```bash
set -euo pipefail
selection="${selection:-all}"
cd "$MASKFILE_DIR"

if [[ "$selection" == "none" ]]; then
  echo "no backends selected"
  exit 0
fi

if [[ "$selection" == "all" ]]; then
  docker compose up -d
else
  # One or several, comma- or space-separated.
  services="$(printf '%s' "$selection" | tr ',' ' ')"
  docker compose up -d $services
fi

docker compose ps
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
