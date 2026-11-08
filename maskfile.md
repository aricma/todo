# aricma-todo Actions

One project, four parts: `lib/` (the markdown todo rules), `cli/` (the
`aricma-todo` binary a pipeline installs), `tui/` (the `lazytodo` screen) and
`testkit/` (what the suites share). Every action here is a plain `cargo` command;
`mask` is only shorthand for it.

## format

> Check rustfmt across the workspace.

**OPTIONS**

-   fix
    -   flags: -f --fix
    -   desc: Rewrite files instead of only checking them

```sh
if [ "${fix:-}" = "true" ]; then
    cargo fmt
else
    cargo fmt --check
fi
```

## lint

> Clippy over the workspace, denying warnings.

```sh
cargo clippy --workspace --all-targets -- -D warnings
```

## test

> The crates' own tests and the use cases.

```sh
cargo test --workspace --all-targets
```

## build

> Build `aricma-todo` and `lazytodo` into tmp/bin.

```sh
cargo build --release --locked
mkdir -p "$MASKFILE_DIR/tmp/bin"
cp "$MASKFILE_DIR/target/release/aricma-todo" "$MASKFILE_DIR/tmp/bin/"
cp "$MASKFILE_DIR/target/release/lazytodo" "$MASKFILE_DIR/tmp/bin/"
```

## use

> Ask the built screen what a person asks first. After `mask build`.

```sh
"$MASKFILE_DIR/tmp/bin/lazytodo" --version
"$MASKFILE_DIR/tmp/bin/lazytodo" --help >/dev/null
```

## ci

> The local mirror of the CI gate: format, lint, test, build and use.

**OPTIONS**

-   fix
    -   flags: -f --fix
    -   desc: Auto-fix formatting and lint instead of only checking them

```sh
set --
[ "${fix:-}" = "true" ] && set -- "$@" -f
$MASK format "$@" && $MASK lint && $MASK test && $MASK build && $MASK use
```
