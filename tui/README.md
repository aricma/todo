# lazytodo

A todo client as a terminal UI, inspired by `lazygit` for its numbered panels and
unmodified single-letter actions, and by
[OPS](https://github.com/aricma/open-productivity-standard) for the document it copies a
todo into. It reads the todos a repository keeps, in its markdown files and in the
`TODO:` comments inside its source, and works through them one key at a time.

```sh
lazytodo .           # the TUI for the current directory
lazytodo path/to    # ... or this one
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

How to install it, and what the repository around it is for, is in the repository's
README, one level up.
