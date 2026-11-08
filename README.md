# aricma-todo

The monorepo for `aricma-todo`: a todo list that lives in markdown, and the programs
that read it. One of them is finished, and this page is about that one — `lazytodo`,
the terminal client. Two more are in the tree and are not done: `lib/`, the rules the
programs share, and `cli/`, the pipeline client. Neither is released, and nothing here
documents them yet.

## lazytodo

![lazytodo open on a repository: panels down the left, the file on the right, keys
across the bottom](./screenshot.png)

`lazytodo` is a todo client as a terminal UI, inspired by `lazygit` for its numbered
panels and unmodified single-letter actions, and by
[OPS](https://github.com/aricma/open-productivity-standard) for the document it copies a
todo into. It reads the todos a repository keeps, in its markdown files and in the
`TODO:` comments inside its source, and puts them on one screen to be worked through.
What it is for is maintenance: a monorepo or a large repository collects todo lines
nobody reads, because reading them costs a `grep` and rewriting one costs an editor and
a jump. Here the work is one key away from the cursor.

### Install

```sh
curl -fsSL https://aricma.app/install.sh | sh -s -- --bin lazytodo
```

The script works out your platform, verifies the checksum of what it downloaded, and
puts `lazytodo` in `~/.local/bin`. `--version 0.1.0` pins a release instead of taking
the latest, and `--install-dir` puts it somewhere else:

```sh
curl -fsSL https://aricma.app/install.sh | sh -s -- --bin lazytodo --version 0.1.0
```

The release page carries the same archives, one per platform, each with a single
checksums file, if you would rather do it by hand. `cargo install --path tui` builds it
from a clone.

### Use

```sh
lazytodo .                   # the TUI for the current directory
lazytodo path/to/TODOs.md    # ... or this one
```

- Numbered panels: `0`–`3` reach the one you want, and a file's row says how much of
  it is done (`5/15`) and how many tags and people it mentions.
- Priorities `A`–`D`, which are also an instruction you can hand an agent: the `A`s
  and `B`s, or only what is assigned to `@kevin`.
- Every tag and every person as a collection to walk, biggest first.
- Search across every todo, and `c` to copy whatever the cursor is on — a todo, a
  path, a heading — with a todo leaving as an OPS document.

### Read

- [tui/README.md](./tui/README.md) — the index: the manual, the RFCs, the glossary,
  the changelog.
- [tui/docs/manual.md](./tui/docs/manual.md) — the manual: the line grammar, the frame,
  the keys, search, limits, messages, history, OPS.
- [tui/docs/rfc/](./tui/docs/rfc/README.md) — one behaviour per RFC, with an index: what
  it does, why that and not the alternative, and what it costs.

If something is wrong or missing, open an issue
[here](https://github.com/aricma/todo/issues).

## What else is in the tree

| part | what it is | state |
| --- | --- | --- |
| `lib/` | the markdown todo rules: parse, edit, check, format, OPS export | the core `lazytodo` is drawn over, not released |
| `cli/` | `aricma-todo` — check, format, export, import; no screen | written, not released |
| `testkit/` | what the suites share: run a binary, read what it said, drive a use case | a dev-dependency |
| `use-cases/` | the use cases as data, one folder per case | how the suites are driven |

A clone builds and tests with `cargo test`; the toolchain is pinned in
`rust-toolchain.toml`, and `mask` actions are in [maskfile.md](./maskfile.md).

## Support

If this saves you some time, there are two ways to say so:

<a href="https://www.buymeacoffee.com/aricma"><img src="https://cdn.buymeacoffee.com/buttons/v2/default-yellow.png" alt="Buy Me a Coffee" width="217" height="60"></a>

[![Support me on Ko-fi](https://storage.ko-fi.com/cdn/kofi2.png?v=3)](https://ko-fi.com/N2N426RWKU)

## License

MIT — see [LICENSE](./LICENSE).

The browser app this library once grew out of is not part of this repository.
