# aricma-todo

A todo list that lives in markdown — plus the two programs that read it: one for
pipelines, one for people.

| part | what it is | how you run it |
| --- | --- | --- |
| `lib/` | the markdown todo rules: parse, edit, check, format, OPS export | a dependency |
| `cli/` | `aricma-todo` — check, format, export, import; no screen, no `ratatui` | `cargo install --path cli` |
| `tui/` | `lazytodo` — the todo client, a terminal UI for reading and editing by hand | `cargo install --path tui` |
| `testkit/` | what the suites share: run a binary, read what it said, drive a use case | a dev-dependency |

Everything it depends on is public — the crates, plus `ops-lib` from a public git
repository — so a clone builds with `cargo test` and no credentials. The toolchain is
pinned in `rust-toolchain.toml`, which `rustup` reads for you.

## The todo client

![lazytodo open on a repository: panels down the left, the file on the right, keys
across the bottom](./screenshot.png)

`lazytodo` is a todo client as a terminal UI: an homage to `lazygit` — the same numbered
panels and unmodified single-letter actions — pointed at todo lines instead of diffs,
with [OPS](https://github.com/aricma/open-productivity-standard) as the document a todo
becomes when it leaves. It reads the todos a repository keeps, in its markdown files and
in the `TODO:` comments inside its source, and puts them on one screen to be worked
through. What it is for is maintenance: a monorepo or a large repository collects todo
lines nobody reads, because reading them costs a `grep` and rewriting one costs an
editor and a jump. Here the work is one key away from the cursor.

- Numbered panels: `0`–`3` reach the one you want, and a file's row says how much of
  it is done (`5/15`) and how many tags and people it mentions.
- Priorities `A`–`D`, which are also an instruction you can hand an agent: the `A`s and
  `B`s, or only what is assigned to `@kevin`.
- Every tag and every person as a collection to walk, biggest first.
- Search across every todo, and `c` to copy whatever the cursor is on — a todo, a path,
  a heading — with a todo leaving as an OPS document.

A release carries one archive per target, each holding the binary, `README.md`,
`CHANGELOG.md`, `LICENSE` and the docs, with a single checksums file beside them:

```
lazytodo-v0.1.0-aarch64-apple-darwin.tar.gz
lazytodo-v0.1.0-x86_64-unknown-linux-musl.tar.gz
```

The Linux archives are statically linked, so they start on any distribution, and the
checksums file lists every archive of its release. `cargo install --path tui` builds it
from source instead.

## Reading

- [tui/README.md](./tui/README.md) — the todo client's index: the manual, the RFCs,
  the glossary, its changelog.
- [tui/docs/manual.md](./tui/docs/manual.md) — the todo client's manual: the line
  grammar, the keys, the frame, the limits, the history.
- [tui/docs/rfc/](./tui/docs/rfc/README.md) — its [RFCs](./tui/docs/rfc/README.md), one
  per behaviour, with an index, and the [glossary](./tui/docs/glossary.md) of the words
  they use.
- [use-cases/](./use-cases/README.md) — the use cases as data: `core/` is the
  document's behaviour on any interface, `cli/` and `tui/` are what only that
  interface can be asked. Each interface declares a driver, and the driver is what
  makes it a test.

`mask` actions are in [maskfile.md](./maskfile.md); every one of them is a plain
`cargo` command you can run without `mask`.

## License

MIT — see [LICENSE](./LICENSE).

The browser app this library once grew out of is not part of this repository.
