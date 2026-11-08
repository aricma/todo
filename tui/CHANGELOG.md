# Changelog

What changed in `lazytodo`, the todo client. Each package here has its own version and
its own tag — this one is `lazytodo-v<version>` — independent of the others.

The newest section is the version a release is cut from, so a release adds one at the
top. There is no Unreleased section: the pipeline refuses to release a version that is
not the newest section here or in `tui/Cargo.toml`.

## 0.1.0

The first release of `lazytodo`: a todo client as a terminal UI, inspired by `lazygit`,
reading the todos in your markdown files and in the comments inside your source. We are
still pre-1.0 but we will add features and fix bugs as they come in.

Start with the [README](https://github.com/aricma/todo/blob/main/README.md) for what it
does and how to install it, then the
[manual](https://github.com/aricma/todo/blob/main/tui/docs/manual.md) for the keys and
the line grammar, and the
[RFCs](https://github.com/aricma/todo/blob/main/tui/docs/rfc/README.md) for why the
screen behaves the way it does.
