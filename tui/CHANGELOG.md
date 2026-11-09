# Changelog

What changed in `lazytodo`, the todo client. Each package here has its own version and
its own tag — this one is `lazytodo-v<version>` — independent of the others.

A change lands under `Unreleased`; a release renames that heading to the version and
starts a new `Unreleased` above it. The pipeline reads the first `##` heading here and
refuses to release a version it does not name, so the rename is part of cutting the
release.

## Unreleased

## Added

The version of `lazytodo` now stands in the lower right corner of the status bar.

## 0.1.1

The first release of `lazytodo` anyone can get: a todo client as a terminal UI, inspired
by `lazygit`, reading the todos in your markdown files and in the comments inside your
source. The number is 0.1.1 because 0.1.0 was published and pulled back again; the
program itself is unchanged. We are still pre-1.0 but we will add features and fix bugs
as they come in.

Start with the [README](https://github.com/aricma/todo/blob/main/README.md) for what it
does and how to install it, then the
[manual](https://github.com/aricma/todo/blob/main/tui/docs/manual.md) for the keys and
the line grammar, and the
[RFCs](https://github.com/aricma/todo/blob/main/tui/docs/rfc/README.md) for why the
screen behaves the way it does.
