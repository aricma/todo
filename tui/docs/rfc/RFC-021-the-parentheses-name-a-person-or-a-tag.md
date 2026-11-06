# RFC-021: The parentheses name a person or a tag

- **Date:** 2026-11-06
- **Related:** [RFC-013](RFC-013-a-todo-is-a-checkbox.md), [RFC-016](RFC-016-a-comment-todo.md), [RFC-019](RFC-019-not-every-todo-is-a-todo.md), [RFC-020](RFC-020-four-words-and-a-marker-per-language.md)

## Context

RFC-020 refused `TODO(bob): `, on the grounds that a second spelling of a field is how two
parsers start disagreeing. That was half right. The spelling was already in the code —
`// TODO(alice): #102 the retry is wrong` is what a repository written against a tracker
looks like — and refusing to read it only makes the tool worse at its one job.

The first draft of this RFC over-corrected in the other direction: it read a bare word in
the parentheses as a person (`TODO(alice): `) and promoted `#102` to a **ticket**, a field
of its own with a colour of its own. Both were wrong, for the same reason seen from two
sides:

- a `#` token is a **tag**. A number somebody filed something under is still a tag, and the
  tag collection already answers "everything about `102`" — a ticket type needed a colour, a
  field, and a key in every export to say nothing the tag list did not already say;
- a bare word in the parentheses says nothing about which of the two it is. `TODO(bob): ` does
  not say whether `bob` is a person or a thing to file the note under, and a rule that has to
  guess has already lost.

So the parentheses are read, and what is inside them has to be something the body already
knows how to say.

## Behaviour

1. A severity word **MAY** be followed by one `@person` or one `#tag` in parentheses and
   then its colon: `TODO(@alice): `, `FIXME(#home): `, `TODO(#102): `. The `)` **MUST** be
   followed immediately by `:`.
2. What is inside **MUST** carry its sigil. A bare word — `TODO(alice): `, `TODO(bob): `,
   `TODO(102): `, `TODO(A): ` — is **not** accepted, and the line **MUST NOT** be a todo at
   all. There is no third reading of a parenthesis to fall back on.
3. The value **MUST** be normalised into the list the body's own token goes into —
   `task.assignees` for a person, `task.tags` for a tag — first in it, in written order, and
   once: `TODO(@alice): ship it @alice` names one person. It **MUST NOT** become a field of
   its own.
4. The parentheses are **MUST NEVER** be a priority. `TODO((A)): ` is not a thing; a
   priority is written after the colon, where the body is (RFC-016 rule 4), so
   `TODO(@alice): (A) pay rent` is a person and a priority.
5. The text of the parentheses **MUST** be preserved in the line's lead and the tool **MUST
   NOT** rewrite it: `task.lead()` is `    // FIXME(@alice): ` (RFC-016 rule 3).
6. `TODO (@alice): ` (space before the parenthesis), `TODO(): ` and `TODO(@alice: ` are
   **not** a todo's word: a space means the word is over, and a parenthesis that closes on
   nothing names nobody. RFC-019's table carries each as a miss.
7. A `#` token in a body **MUST** stay a tag whatever it looks like: `#102`, `#2024`,
   `#PROJ-404`, `#home` and `#worst-bug-of-the-century` are one kind of thing, with one
   colour, one collection and one entry in an OPS document's `tags`. A number is not a
   second kind of `#`.

## Why not the alternative

**A `Ticket` type.** It was one commit long, and it is the useful half of this RFC: a
number in a `#` token is not a different namespace, it is the same namespace with digits in
it. A ticket field would have needed its own colour, its own line in every export (the OPS
spec has no common key, so a namespaced one), and its own rules for "two tickets on a line" —
all to answer a question the tag collection already answers. Deleted, with the colour.

**Accepting a bare word in the parentheses** — `TODO(alice): ` — and calling it a person.
It is one less character for the writer and one indistinguishable case for the reader, and
the character is not being saved: the body already writes `@alice`, so the version without
the sigil is a second spelling nobody needs. Worse, `TODO(A): ` would then be a person
called `A` sitting exactly where a priority is written in every other todo.

**Reading the parentheses as a priority when they hold one letter.** Same collision from
the other side. Position decides: after the colon is the body, and the body is where a
priority lives.

**A ticket without the `#`,** as `TODO: PROJ-404 the retry is wrong`. The `#` is what says
"this token is not a word of the title". Without it, `PROJ-404` is a hyphenated word in a
sentence.

## Consequences

- **`#102` is a tag again.** It is in the tag count, in the `/102` collection, and in an
  OPS document's `tags` — which is what it was before this RFC, and what a `#` token has
  meant in this tool from the start.
- **`TODO(bob): ` is not a todo.** RFC-019's table records it, with the other three
  malformed parentheses, and `TODO(@bob): ` / `TODO(#home): ` are listed.
- **A note with a bare word in the parentheses disappears from the tree.** It is visible in
  the file, dim, on the line a person wrote — the same accepted cost as every other miss in
  RFC-019, and the fix is one character.
- **`ops::export` reads the `Task`, not the body.** The person or tag in the parentheses
  lives in the part of the line the tool does not own, so a second parse of the body would
  export neither. This stays from the earlier draft, and it is the reason the round trip
  works.
- **RFC-020's rule 17 is still reversed and rule 18 is not.** A person between the word and
  the colon is read (with a sigil), and a ticket is still not a field: both statements there
  are now true as written.

## How it is proven

- `lib/src/doc/parse.rs` — `a_word_may_name_who_it_is_for`,
  `a_word_may_file_it_under_a_tag`, `what_the_word_files_is_in_the_body_s_own_list`,
  `a_parenthesis_without_a_sigil_is_not_a_word`, and the amended
  `the_rule_is_narrow_and_its_misses_are_known` (two hits and five misses for this rule).
- `lib/src/ops/export.rs` — `a_person_named_before_the_colon_is_one_of_the_assignees`, and
  the copy round trip that still carries a tag, a person and a priority.
- `use-cases/tui/parentheses-name-a-person-or-a-tag` — the whole thing through a real pty:
  `sigil.rs  0/2  #2  @2` in the file panel with `#102  0/1` and `@alice  0/2` as
  collections, and the bare-word line on screen but not counted.

## Open questions

- **A collection for a ticket number.** `#102` is a tag and therefore a collection, so this
  is already answered — it is `/102`, in the tags of the panel header. Whether a number tag
  should be *shown* differently is the same question as below, and nothing filters on it.
- **Severity on the screen.** A `FIXME:` reads as one because the word is still in the line;
  nothing counts or folds by it (RFC-020's open question, unchanged).
- **Two people or two tags before the colon.** `TODO(@alice,@bob): ` and `TODO(#a #b): ` are
  not read. One value is the shape every tracker uses, and a list would need its own rule
  about separators.
