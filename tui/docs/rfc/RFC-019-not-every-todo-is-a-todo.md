# RFC-019: Not every line with a `TODO` in it is a todo

- **Date:** 2026-10-16
- **Related:** [RFC-007](RFC-007-search.md), [RFC-015](RFC-015-sources-and-kinds.md), [RFC-016](RFC-016-a-comment-todo.md), [RFC-017](RFC-017-the-tree-is-what-git-sees.md), [RFC-020](RFC-020-four-words-and-a-marker-per-language.md)

## Context

Finding the notes a person left in the code should be easy: they wrote `TODO:` and
everyone knows what it means. It is not easy, because whether a line is a comment is a
fact of the *language* — strings, escapes, raw strings, heredocs, nesting, templates, a
`#` inside a shell string, a `"` inside a Rust string — and this tool has no grammar for
any of them. A real answer is a parser per language, kept current forever.

A line-matching rule has two ways to be wrong, and they cost differently:

- a **miss** — a real todo is not listed. The file is not in the tree, the count is one
  low, and a person who does not already know the note exists will never see it.
- a **false positive** — a line *about* a todo is listed. The count is one high, a file
  appears in the tree for no reason, and every row has to be read before it is trusted.

This tool chooses **precision**: a list you must filter by hand is not a list. A miss is
still visible — the note is in the file, in front of the person who wrote it — while a
false positive is invisible, because nothing in the row says "this is not really a todo".

That choice was not made on paper. Every condition below was added after this repository
listed its own files for todos they did not have:

| what it listed | why | the condition that stopped it |
| --- | --- | --- |
| `tui/src/ui/hints.rs` | a doc comment reading ``/// the todos are `TODO:` comments`` | a marker must stand in front of the word |
| `lib/src/doc/parse.rs` (16 rows) | its own test fixtures: `Doc::parse_comment("// TODO: x")` | a quote in front of the marker means the marker is quoted |
| a test's own comment | `// a file that no longer holds a todo: the panels …` | capitals |

Three conditions, three false positives, and the fourth class — prose that writes an HTML
comment marker without quoting it — is still there and is admitted below.

## Behaviour

1. A hit **MUST** be a fact about a *line*, never a claim about a *comment*. The tool
   says "this line holds a `TODO:` that looks like a note", not "this is a comment".
2. All of the following **MUST** hold, or the line is not a todo:
   - the word is one of `TODO`, `FIXME`, `HACK`, `XXX` — four, since RFC-020 — **in
     capitals**, followed by `:`. `todo:`, `Todo:`, `TODO(x):` and `TODO` with no colon
     are not todos, and a word that is the tail of a longer one (`NOTTODO:`) is not
     either;
   - at least one character after the colon that is not a space (RFC-016);
   - the text ending where the word begins **MUST** hold a marker the file's language
     starts a comment with — `#`, `/`, `-`, `;`, `%` or `*` where the language is unknown,
     which covers `#`, `//`, `///`, `/*`, `--`, `;`, `%`, `<!--` and a `*` continuing a
     block — or be nothing but indent, which is how a list in a plain `.txt` file is
     written;
   - that text **MUST NOT** hold `"`, `'` or a backtick: a marker behind a quote is a
     quoted marker, inside a string literal or inside prose about code.
3. A line with more than one severity word **MUST** yield the first one that satisfies
   the rule, and only that one.
4. The costs of each condition **MUST** be documented where a person will look for them,
   and the table in this RFC **MUST** be proven by a test, so a rule change fails the
   test before this document starts lying.
5. A file with no hit **MUST NOT** be listed (RFC-017), so a file's absence from the tree
   is evidence of nothing but the rule's silence.

## The table

Normative, and asserted line for line by `the_rule_is_narrow_and_its_misses_are_known` in
`lib/src/doc/parse.rs`. It is stated for `Syntax::Any` — a file whose language is unknown —
which is what `Doc::parse_comment` means.

| line | listed | why |
| --- | --- | --- |
| `    // TODO: pay rent` | yes | the commonest thing there is |
| `# TODO: pay rent` | yes | shell, python, yaml |
| `-- TODO: pay rent` | yes | sql, lua, haskell |
| ` * TODO: pay rent` | yes | continuing a `/* … */` |
| `/* TODO: pay rent */` | yes | the block's opening line |
| `<!-- TODO: pay rent -->` | yes | html, xml |
| `TODO: pay rent` | yes | a list in a `.txt`, column 0 |
| `  TODO: pay rent` | yes | the same, indented |
| `let a = 1; // TODO: pay rent` | yes | a trailing note, no quote in front |
| `let x = 1; //TODO: pay rent` | yes | the space is not required |
| `// TODO: a "quoted" word` | yes | the quote is the todo's own text |
| `// FIXME: pay rent` | yes | since RFC-020: a thing that is broken is a thing to do |
| `// HACK: pay rent` | yes | the same |
| `// XXX: pay rent` | yes | the same |
| `let a = "x"; // TODO: pay rent` | **no** | a quote in front: the tool cannot tell which side of it the note is on |
| `"// TODO: pay rent"` | **no** | quoted marker |
| ``/// a `TODO:` note`` | **no** | quoted marker |
| `// todo: pay rent` | **no** | lowercase is a sentence about a todo |
| `// Todo: pay rent` | **no** | the same |
| `// TODO pay rent` | **no** | no colon |
| `// TODO(bob): pay rent` | **no** | the colon is not the word's colon; `(bob)` means *who*, not *how urgent* (RFC-016) |
| `// NOTTODO: pay rent` | **no** | the word is the tail of a longer word |
| `// TODO:` / `// TODO:   ` | **no** | promises something, says nothing |
| `/// we use <!-- TODO: --> in prose` | **yes** | *the false positive that stays*: an unquoted `<!--` in a sentence is indistinguishable from an HTML comment by any line rule |

### What a known language changes

RFC-020 reads the marker from the file's extension, so the same line is a todo in one file
and not in another. Asserted by `only_the_marker_this_language_uses_opens_a_comment`.

| line | listed | why |
| --- | --- | --- |
| `# TODO: pay rent` in a `.py` | yes | `#` is a comment there |
| `# TODO: pay rent` in a `.rs` | **no** | `#` is an attribute there, and the line's own language says so |
| `// TODO: pay rent` in a `.rs` | yes | the marker that language does use |
| `TODO: pay rent` in a `.txt` | yes | no marker, no language: a notes list |
| `TODO: pay rent` in a `.sh` | **no** | in a known language a bare line is a syntax error |

## Why not the alternative

**The loose rule — any `TODO:` anywhere, in any case.** It was the first version, and it
was free to write: one `find`. It listed six of this repository's own source files for
todos none of them had, which is how the three conditions were found. A reader who scrolls
past "todos" that are sentences learns to skim the panel, and a panel that is skimmed is
not read.

**Requiring the marker to start the line.** It would kill the last false positive in the
table and every false positive like it, since a trailing note would no longer count. The
cost is real: `let a = 1; // TODO: revisit this` is a note about the line it follows, and
that is the note's whole value.

**A per-language grammar.** Correct, and a different product: a linter for languages this
tool does not otherwise understand. It also ages badly — grammars change, and every one of
them would have to be vendored into a todo reader.

**A regex over marker and word** — `^\s*(//|#|--|;|%|/\*|<!--)\s*TODO:`. It is the
same rule written tighter, with the same misses (a trailing note is not at the start), and
it cannot express the quote condition at all, which is the one that removed most of the
noise.

**Reading the whole tree as text and searching it.** That is search (RFC-007) — and today
search looks at todos, so it inherits these same misses instead of covering them. A
grep-style view of every occurrence is a real idea and is an open question below, not a
quiet behaviour of the tree.

## Consequences

- **Every count is a lower bound.** A file's `3` means three notes the rule can see. The
  header does not apologise for it, because a count that says "at least" is a count nobody
  reads.
- **A miss is discoverable.** The note is in the file, at the line the person wrote it; the
  escape from a miss is reading the code, which is where the note already is.
- **The rule is two small steps, and the words are a table.** Where a file's comments are
  is `doc::comment`; which one is a todo is `todo_in`, and the four words are
  `Severity::ALL` (RFC-020). The table's test is still the guard, and changing a condition
  is still a small diff plus an RFC amendment.
- **The tool's own test fixtures are not todos** — and a real project's fixtures will be,
  if they put `TODO:` at the start of a comment line. There is no way to know a fixture
  from a note, and this RFC does not pretend otherwise.
- **A trailing note after quoting code is lost.** `let a = "x"; // TODO: …` is invisible
  to the tree. The workaround is one line: put the note on its own line.

## How it is proven

- `the_rule_is_narrow_and_its_misses_are_known` (`lib/src/doc/parse.rs`) — the whole
  table above, both directions, plus the admitted false positive.
- The repository itself: before the three conditions, `scan` returned seven comment todo
  files; after them, one — `use-cases/tui/read-a-comment-todo/fixture-retry.rs`, which
  really does hold two `TODO:` comments on their own lines.
- `use-cases/tui/read-a-comment-todo` reads a file of that shape end to end through a pty
  (RFC-018).

## Open questions

- Should there be an explicit loose mode — a `--loose` flag or a "show every `TODO:`" view —
  for the person who would rather filter than miss? It is a second rule to keep true, and
  it is not needed until someone is caught by a miss in practice.
- ~~Is `FIXME:` a todo?~~ Answered by RFC-020: yes, along with `HACK:` and `XXX:`, and the
  task keeps which word opened it.
- Should a file that holds `TODO` lines the rule refused be marked, so a person can see
  that the tool looked and declined? It would say, in the tree, that the rule is a guess —
  which this RFC says in prose and the screen does not.
- Should the capital rule be per-file, for a codebase whose style guide writes `todo:`?
  Every condition here is a convention, and conventions vary by repository.
