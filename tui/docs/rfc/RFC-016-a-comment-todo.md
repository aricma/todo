# RFC-016: A comment todo is one line of code

- **Date:** 2026-10-16
- **Related:** [RFC-013](RFC-013-a-todo-is-a-checkbox.md), [RFC-014](RFC-014-a-copied-todo-is-a-document.md), [RFC-015](RFC-015-sources-and-kinds.md), [RFC-018](RFC-018-reading-a-comment-todo-file.md), [RFC-019](RFC-019-not-every-todo-is-a-todo.md), [RFC-020](RFC-020-four-words-and-a-marker-per-language.md), [RFC-021](RFC-021-who-it-is-for-and-which-ticket.md)

## Context

A `TODO:` comment is the oldest todo format there is, and every codebase disagrees about
its details: `TODO(x):`, `TODO [x]`, `// TODO` without a colon, a `<!-- TODO: -->` in a
markdown file, a `TODO:` inside a string literal. (The four words — `TODO`, `FIXME`,
`HACK`, `XXX` — and the marker each language starts a comment with are RFC-020's subject.)
Parsing a code
file properly — comments, strings, nesting — is a compiler's job, and this is a todo
reader.

So the rule has to be one line of text matching, and the honest thing is to say exactly
what it matches and what it therefore gets wrong. **That is RFC-019's subject**: the three
conditions a line must satisfy (`TODO:` in capitals, a comment marker in front of it, no
quote between them), the false positives that taught them to us, and the misses they cost.
This RFC is about the *grammar* of what a hit contains.

## Behaviour

1. In a **comment todo file** — any file the walk reads that is not `.md` — a todo **MUST** be
   a line that RFC-019 and RFC-020 accept as one, and one todo per line at most: the first
   accepted severity word on it. Everything this RFC says about a hit's shape assumes that
   rule, and nothing here widens it.
2. A line with a severity word and nothing after it is **not** a todo: it promises something
   and says nothing. (A document's empty `- [ ]` *is* a todo, because the row is where a new
   todo waits; a comment has no such row.)
3. Everything in front of the body **MUST** be preserved byte for byte, and **MUST** be
   shown as the todo's lead: the indentation, the comment marker, the severity word, and
   whatever the language puts there — `    // TODO: `, `# FIXME: `, `/* HACK: `. The tool
   does not own the comment syntax and never rewrites it.
4. The body is the text after the severity word, its colon, and the one space that follows.
   It is read by the same rules as a todo document's body, and **MUST** be: a priority token
   `(A)`–`(D)` at the start, `#tags`, `@people`, a `#102` ticket anywhere (RFC-021). That is
   how a comment todo gets a priority — **`TODO: (A) fix the retry`** — and it is why the
   priority is written *after* the colon rather than inside the comment marker.
   A person **MAY** also be named between the word and its colon — `TODO(alice): ` — and is
   then one of the body's `@people`, never a second field, and never a priority (RFC-021).
5. Editing a comment todo **MUST** rewrite only the body. The code in front of it and
   after it on the same line is untouched.
6. A comment todo **MUST** be able to be: edited, copied, priced, deleted. It **MUST NOT**
   be checked, added to by the tool, or moved:
   - **no check** — a comment has no checkbox, and its only "done" is being removed;
   - **no add** — where in the code the thought belongs is the person's decision, and the
     comment syntax is the language's;
   - **no move** — the line sits next to the code it is about, and moving it would move
     the note away from its subject.
7. A **markdown file MUST NOT** be read for comment todos, even when it holds
   `<!-- TODO: … -->` or a `TODO:` inside a code block. Its todos are its checkboxes, and
   a second kind inside one file is a promise this version does not keep.
8. A todo comment's `title` **MAY** keep a trailing comment terminator (`fix it */`),
   because the tool does not know comment syntax. Copying such a todo into a document
   carries the terminator with it, and the person edits it out.
9. A comment todo **MUST** join collections like any other todo: a `#tag` in the comment
   is a tag, and the collection shows the todo in its file's order.

## Why not the alternative

**A real comment parser per language.** It would have to know strings, escapes, raw
strings, heredocs, nesting and templates — a table of languages, kept current forever —
to avoid the mistakes the simple rule still makes (a `TODO:` in a string the language
does not quote with `"` — the marker rule already skips `let s = "TODO: x";` — and a
folded multi-line comment). This tool would then be a linter for the languages it does not
pretend to understand.

**`TODO(name):` as the priority slot.** It is the ECMAScript convention and it means *who*,
not *how urgent*. Overloading it would make two codebases' meanings collide, and a
priority written by this tool would read as an owner to every other tool that looks.

**A priority on the checkbox model** — `- [ ] (A) text` — does not exist in a comment, so
`TODO: (A) text` was chosen because it reuses the one body syntax the tool already parses,
without inventing a second one.

**Reading comment todos in markdown too.** Everyone who wants a comment todo in a
document writes prose instead, and the two rules would fight over the same file: a
`- [ ] …` inside a `<!-- -->` example, a `TODO:` inside a fenced code sample. Not in this
version; a later RFC may narrow it.

## Consequences

- The todo's `lead` is no longer derivable from indent and bullet; it is stored for a
  comment todo, and every write path (edit, priority, format) goes through it. The
  library's column arithmetic moved to "where the lead ends", which is one rule for both
  kinds.
- `findings` and `format` are about documents only, so a comment todo file reports nothing and
  `--format` never rewrites a comment.
- What is still known and wrong about a hit that RFC-019 does accept: a multi-line
  `/* … */` comment is one long body on its first line, and the closing `*/` stays in the
  body, because the tool does not know it is a terminator. `--check` reports neither; the
  misses and false positives of the finding rule are RFC-019's.
- A test proves the body rules are shared: `TODO: (A) fix #infra @dave` reads as
  priority A, one tag and one person, exactly like `- [ ] (A) fix #infra @dave`.

## Open questions

- Should a folding multi-line comment (`/* TODO: …` on one line, the rest below) be
  treated as one todo with several lines? It is the one common shape the one-line rule
  gets visibly wrong.
- Is `FIXME:` a todo? Some teams treat it as stronger than a `TODO:`; others as a
  different list. Today it is not a todo at all — the word is not in the rule.
- Should `--check` name the string-literal case, at least for the comment markers the tool
  could recognise cheaply (`//`, `#`, `--`, `;`)?
