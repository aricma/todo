# Changelog

What changed in `aricma-todo`, the command a pipeline runs. This package has its own
version and its own tag (`aricma-todo-cli-v<version>`), independent of the other
packages here.

The newest section is the version a release is cut from, so a release adds one at
the top. There is no Unreleased section: the pipeline refuses to release a version
that is not the newest section here or in `cli/Cargo.toml`.

## 0.1.0

The first release. Everything below is new, so nothing here is a fix.

- `check` reports every todo-rule violation and exits non-zero, and never writes.
- `format` fixes the checkbox case and the priority letter case.
- `export` writes every todo as a single OPS document.
- `import` appends the todos of an OPS document to a file.
- The same rules the screen uses, from the same library, so a pipeline and a person
  cannot disagree about what a line means.
