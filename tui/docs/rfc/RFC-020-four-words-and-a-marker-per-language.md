# RFC-020: Four words, and the marker each language uses

- **Date:** 2026-11-06
- **Related:** [RFC-015](RFC-015-sources-and-kinds.md), [RFC-016](RFC-016-a-comment-todo.md), [RFC-017](RFC-017-the-tree-is-what-git-sees.md), [RFC-018](RFC-018-reading-a-comment-todo-file.md), [RFC-019](RFC-019-not-every-todo-is-a-todo.md)

## Context

One function did two jobs. `comment_line` in `lib/src/doc/parse.rs` looked for the literal
`TODO:` in a line, and the condition that made it a comment — *a marker ends the text in
front of the word* — was one more thing to find in the same pass. That worked while there
was one marker set and one word, and it put two questions in one answer:

- **Where are this file's comments?** A fact about the language.
- **Which of them is a todo?** A fact about the line.

They were fused, so neither could be changed without changing the other: widening the
words meant re-checking the marker rule for every word, and narrowing the markers meant
finding the word without a rule to lean on. The seam a reader of the code would expect —
comments first, then todos in them — was not there.

There is a second reason. The literal `TODO:` is one convention among four, and `FIXME:` is
the one every codebase writes for "this is broken". The marker set is one convention among
a dozen: `#` is a comment in Python and an attribute in Rust, `--` is a comment in SQL and
subtraction in C, `//` is a comment in Go and a URL in a YAML string. One set of markers for
every file is a compromise every language loses.

## Behaviour

### The two phases

1. Reading a comment todo **MUST** be two steps, in two places:
   - `lib/src/doc/comment.rs` — **where a file's comments are**. `comments(lines, syntax)`
     returns one `Comment { line, at }` per line that carries one, and that is the whole of
     what it decides.
   - `lib/src/doc/parse.rs` — **which one is a todo**. `todo_in(line, comment)` applies the
     rule to the comment's text and builds the `Task`.
2. Neither step **MUST** be able to answer the other's question: `comment.rs` does not know
   what a todo is, and `parse.rs` does not know what a marker looks like.
3. One constructor **MUST** take the path, so the two derivations cannot disagree: a
   `Doc`'s source is `Source::of(path)` and its markers are `Syntax::of(path)`, both from
   the same name, in `Doc::parse_at(path, text)`.

### What is not a parser

4. `comment.rs` **MUST NOT** be described as parsing a language, and **MUST NOT** pretend
   to: it finds the **first character** the syntax starts a comment with. Not the whole
   marker, not a word, not a string: `//` where a `/` already is would be a second rule to
   keep in step with the first, and strings are RFC-019's accepted miss, not this RFC's fix.
5. A **block comment MUST NOT** be read. A todo is one line, and a region that spans lines
   needs an end this tool's write model does not have — editing, undo, copy and `--check`
   all address a line. It is an open question below and a `(D)` item in `tui/TODOs.md`, not
   a quiet limitation.

### Which marker opens a comment

6. A comment **MUST** start at the first character the file's syntax accepts, and the
   syntax **MUST** come from the file's extension, or from its name when it has none.
7. The table is short on purpose — three shapes cover almost every language a todo is
   written in, and everything else is permissive rather than wrong:

| syntax | markers | files |
| --- | --- | --- |
| `Slash` | `/` | `.rs .js .mjs .cjs .jsx .ts .tsx .go .c .h .cc .cpp .cxx .hpp .hh .java .kt .kts .swift .cs .scala .dart .php .proto .zig .v .groovy .sol .cu` |
| `Hash` | `#` | `.py .pyi .sh .bash .zsh .fish .rb .pl .pm .r .nix .nim .ex .exs .jl .cr .ps1 .yaml .yml .toml .ini .cfg .conf .mk`, `Makefile GnuMakefile Dockerfile Containerfile Gemfile Rakefile Vagrantfile Justfile Procfile`, and an extensionless dotfile (`.gitignore`, `.env`) |
| `Dash` | `-` | `.sql .lua .hs .elm .ada .adb .ads .vhd .vhdl` |
| `Any` | `# / - ; % *`, and a line with no marker at all | everything else — an unknown extension, a `.txt`, a file with no extension |

8. A file whose syntax is `Any` **MUST** accept a line with no marker, because a notes file
   writes its todos as bare lines and that is a format, not a mistake. A file whose
   language is known **MUST NOT**: `TODO: x` alone in a `.rs` file is a syntax error in
   Rust, not a note.
9. `Doc::parse_comment` **MUST** mean `Syntax::Any`. It is what a caller that does not know
   the file's name can honestly claim, and it is the syntax RFC-019's table is written
   against.

### Which word opens a todo

10. The word **MUST** be one of four, **in capitals**, followed by `:`, and at least one
    character after the colon that is not a space (RFC-016):

| word | means |
| --- | --- |
| `TODO` | a thing to do |
| `FIXME` | a thing that is wrong |
| `HACK` | a thing done knowingly |
| `XXX` | a thing to look at again |

11. A word **MUST** start a word: `NOTTODO:`, `A_TODO:` and `XFIXME:` are longer words that
    happen to end in one, and are not todos.
12. Case-insensitivity **MUST NOT** be added. `fixme:` is a sentence that talks about one,
    and RFC-019 chose a rule that guesses narrow — see the cost section.
13. The word **MUST** be inside the comment, but **MUST NOT** be required to sit right after
    the marker: `// note: FIXME: the retry is wrong` is a todo, because both halves are
    inside the comment. This is the one place RFC-019's rule is **widened**, and it widens
    nothing outside a comment.
14. The quote rule **MUST** still hold over the whole line in front of the word, so a
    `TODO:` inside a string is still invisible: `let a = "x"; // TODO: pay rent` stays a
    miss. The marker rule and the quote rule keep their old jobs; what changed is that the
    marker is no longer required to be *adjacent*.
15. The first word on the line that satisfies all of the above **MUST** win, and only that
    one, as before (RFC-019 rule 3).
16. The word **MUST** be preserved byte for byte in the line, kept in the task's lead, and
    reported as `Task::severity`. The tool does not rewrite a comment's marker or its word
    (RFC-016 rule 3), so the screen shows `// FIXME:` without the model having to render it.

### What was not added here, and what RFC-021 added afterwards

17. `TODO(alice):` was **not** read as an assignee in this RFC: a body already has
    `@alice` (RFC-016 rule 4), and a second spelling of one field is how two parsers start
    disagreeing about the same line. **Partly reversed by
    [RFC-021](RFC-021-the-parentheses-name-a-person-or-a-tag.md)**: the parentheses are
    read when what is in them is `@alice` or `#home` — the sigil is what makes it a
    spelling of an existing field and not a third thing. A bare `TODO(alice): ` still is
    not read.
18. A ticket is **not** a field of its own, and this stands. `#102` is a tag: a second kind
    of `#` token would have to be re-taught to collections, findings, copy and search to buy
    nothing but a label. RFC-021 tried it for one commit and deleted it.

## Why not the alternative

**One function, one `find` (what it was).** It was smaller and it could not grow: every
word, every marker set and every language had to be argued against a rule expressed as
"the last non-space character before the word". The two-phase version states each question
where a reader looks for it, and the second phase inherited the first for free.

**A case-insensitive match, `(?i)`.** It is what the first draft of this did, and it is
tempting because `fixme:` reads the same. It brings back the class of false positive that
made RFC-019 exist: a sentence about a todo is written in lower case far more often than a
todo is. Capitals stay, and the cost is a codebase whose style guide writes `todo:` — an
open question below, and the same one RFC-019 asked.

**A per-language string parser instead.** It would fix the accepted miss of RFC-019 —
`let a = "x"; // TODO: …` — and it is the reason RFC-016 rejected a real parser: a table of
escapes, raw strings, heredocs, templates and nesting, kept current forever, to avoid a
mistake that is visible in the file. The marker table is the smallest useful piece of a
lexer, and it stops at the marker on purpose.

**A `Severity` that filters.** Sorting or folding by severity is a feature nobody asked
for, and the counts would need a second kind of panel header. The word is stored, and it is
shown because it is still in the line.

## Consequences

- **A marker this language does not have is now a miss.** `# TODO: x` in a `.rs` file was
  listed and no longer is; the same line in a `.py` file still is. RFC-019's table is
  restated: it is written against `Syntax::Any`, and a second table there says what the
  per-language rule adds and takes away.
- **A bare `TODO: x` in a known language is now a miss.** In an unknown one it is still
  listed. This is the same statement: the file's language says what a comment looks like.
- **`FIXME:`, `HACK:` and `XXX:` are todos now.** RFC-019's open question — *is `FIXME:` a
  todo?* — is answered yes, and the answer is a `Severity` on the task rather than a second
  list.
- **`TODO(@bob):` and `TODO(#home):` are todos too, as of RFC-021.** The parentheses name a
  person or a tag, with the sigil their body spelling already has; a bare `TODO(bob): `
  stays a miss, and RFC-019's table records all of it.
- **A note in the middle of a comment is found.** `// note: FIXME: x` and
  `/* note — TODO: x */` were missed; the marker only had to be adjacent. That is the
  widening this RFC buys, and it is inside a comment, which is where precision is cheap.
- **The repo's own source is still clean.** The condition that lists this repository for
  todos it does not have did not come back: doc comments quoting `TODO:` hold a backtick,
  which the quote rule still refuses.
- **Two files to read instead of one.** `comment.rs` for where comments are, `parse.rs` for
  what a todo is — and no file that has to be edited for both questions at once.
- **The CLI reads through the same constructor.** `check`, `format` and `export` were
  parsing every file it was given as a todo *document*, because `Doc::parse` was the only
  constructor they knew: a comment todo file exported nothing, and `--format` could have
  rewritten a `- [ ]` line that lives inside code. They take the path now, like the tree
  does (rule 3).

## How it is proven

- `doc::comment` — `the_c_family_reads_a_double_slash`, `shells_and_scripts_read_a_hash`,
  `a_hash_in_a_rust_file_is_not_a_comment_and_the_slash_still_is`,
  `a_bare_line_is_a_comment_only_where_the_language_is_unknown`,
  `every_comment_on_the_file_is_returned_in_order`.
- `doc::parse` — `every_severity_word_opens_a_todo_and_says_which_it_is`,
  `a_marker_in_the_middle_of_a_comment_still_counts`,
  `the_severity_word_has_to_start_a_word`,
  `only_the_marker_this_language_uses_opens_a_comment`.
- `model::severity` — `every_word_opens_its_own_colon_and_nothing_else`,
  `a_lowercase_word_is_a_sentence_and_not_a_severity`, `no_word_is_a_prefix_of_another`.
- RFC-019's `the_rule_is_narrow_and_its_misses_are_known` — amended, and still the guard on
  the table in that RFC.

## Open questions

- **Block comments.** `/* … */` and `<!-- … -->` spanning lines are the obvious next step
  and the expensive one: a todo would need an `end_line`, and editing, undo and copy would
  need a range. Until then a block comment's first line is read and its second is not, which
  is the same shape of miss RFC-019 already admits. Tracked as a `(D)` item in
  `tui/TODOs.md`.
- **The word after a marker on the same line as code.** `let a = 1; /* FIXME: x */` works;
  `/* FIXME: x` on its own line with the text continuing below does not. That is the same
  question as above.
- **A repository that writes `todo:` in lower case.** RFC-019 asked it and this RFC did not
  answer it: a per-file or per-repository setting is a second rule to keep true, and nothing
  has been caught by it yet.
- **Severity on the screen.** The word is in the line, so the panel shows it. Whether a
  `FIXME` should read differently from a `TODO` — an icon, a count, a fold — is a design
  question with no evidence yet, and nothing filters on `severity`.
