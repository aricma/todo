# aricma-todo Actions

One project, four parts: `web/` (the Leptos SPA, built by trunk), `lib/` (the
markdown todo rules), `cli/` (the `aricma-todo` binary a pipeline installs) and
`tui/` (the `aricma-lazytodo` screen). `web/` is its own cargo workspace because
it is wasm-only; `lib/`, `cli/` and `tui/` share the workspace at this level.

Every action here is the project's, and delegates to the part that owns it. The
web app's own actions live in [web/maskfile.md](./web/maskfile.md) — run that one
directly (`cd web && mask preview`) when working on the SPA alone. Standard:
[../DOCS/maskfile-spec.md](../DOCS/maskfile-spec.md).

## format

> Check rustfmt across the native crates, then the web app's own formatting.

**OPTIONS**

-   fix
    -   flags: -f --fix
    -   desc: Rewrite files instead of only checking them

```sh
if [ "${fix:-}" = "true" ]; then
    cargo fmt
    (cd web && cargo fmt)
else
    cargo fmt --check
    (cd web && cargo fmt --check)
fi
```

## lint

> Clippy over the native crates (denying warnings), then the web app's lint.

```sh
cargo clippy --all-targets -- -D warnings
(cd web && mask lint)
```

## test

> The native crates' tests (lib, cli, tui, the use cases), then the web app's.

```sh
cargo test
(cd web && mask test)
```

## build

> Build `aricma-todo` and `aricma-lazytodo` into tmp/bin, then the wasm app.

```sh
cargo build --release
mkdir -p "$MASKFILE_DIR/tmp/bin"
cp "$MASKFILE_DIR/target/release/aricma-todo" "$MASKFILE_DIR/tmp/bin/"
cp "$MASKFILE_DIR/target/release/aricma-lazytodo" "$MASKFILE_DIR/tmp/bin/"
(cd web && mask build)
```

## preview

> Serve the SPA. Its server, ports and backends are documented in web/maskfile.md.

```sh
cd web && mask preview
```

## ci

> The local mirror of the CI gate: format, lint, test and build.

**OPTIONS**

-   fix
    -   flags: -f --fix
    -   desc: Auto-fix formatting and lint instead of only checking them

```sh
set --
[ "${fix:-}" = "true" ] && set -- "$@" -f
$MASK format "$@" && $MASK lint && $MASK test && $MASK build
```
