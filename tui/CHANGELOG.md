# Changelog

What changed in `lazytodo`, the terminal client. This package has its own version
and its own tag (`lazytodo-v<version>`), independent of the other packages here.

The newest section is the version a release is cut from, so a release adds one at
the top. There is no Unreleased section: the pipeline refuses to release a version
that is not the newest section here or in `tui/Cargo.toml`.

## 0.1.0

The first release of `lazytodo`, the terminal client: the todos a repository already
has, in its markdown files and in the `TODO:` comments inside its source, on one
screen and worked through one key at a time.

What it does is in the [README](https://github.com/aricma/todo/blob/main/tui/README.md),
and the behaviour of the screen — one decision at a time, with what it costs — is in
the [RFCs](https://github.com/aricma/todo/blob/main/tui/docs/rfc/README.md).
