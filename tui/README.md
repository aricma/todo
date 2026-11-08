# lazytodo

A todo client as a terminal UI, inspired by `lazygit` and [OPS](https://github.com/aricma/open-productivity-standard).
It reads the todos a repository keeps, in its markdown files and in source code comments.

```sh
lazytodo .                   # the TUI for the current directory
lazytodo path/to/TODOs.md    # ... or this one
```

## Where to read

- [docs/manual.md](./docs/manual.md) — the manual: the line grammar, which files it
  opens, the frame, the two layouts, every key, search, limits, where messages go,
  staying current, history, OPS.
- [docs/rfc/](./docs/rfc/README.md) — one behaviour per RFC, with an index: what the
  program does, why that and not the alternative, and what it costs.
- [docs/glossary.md](./docs/glossary.md) — the words the screen uses, and what each
  one means. An RFC uses those names or it is wrong.
- [CHANGELOG.md](./CHANGELOG.md) — what each release changed.
