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

So the parentheses are **text the line carries**, and they are read as a field only when what
is inside them is something the body already knows how to say. What is not understood is left
where it was written: a parenthesis nobody recognises is not a reason to throw the note away.

## Behaviour

1. A severity word **MAY** be followed by one `@person` or one `#tag` in parentheses and
   then its colon: `TODO(@alice): `, `FIXME(#home): `, `TODO(#102): `. The `)` **MUST** be
   followed immediately by `:`.
2. A parenthesis that holds neither **MUST NOT** stop the line being a todo, and **MUST NOT**
   be guessed at: `TODO(bob): `, `TODO(A): `, `TODO(102): ` and `TODO(see the note): ` are
   todos whose parentheses say nothing this tool understands. The text stays in the lead, no
   assignee and no tag is read from it, and nothing about the line changes (RFC-019's rule
   that a hit is a fact about a line, and the tool does not rewrite what it does not own).
3. The value **MUST** be normalised into the list the body's own token goes into —
   `task.assignees` for a person, `task.tags` for a tag — first in it, in written order, and
   once: `TODO(@alice): ship it @alice` names one person. It **MUST NOT** become a field of
   its own.
4. The parentheses are **MUST NEVER** be a priority. `TODO((A)): ` is not a thing; a
   priority is written after the colon, where the body is (RFC-016 rule 4), so
   `TODO(@alice): (A) pay rent` is a person and a priority.
5. The text of the parentheses **MUST** be preserved in the line's lead and the tool **MUST
   NOT** rewrite it: `task.lead()` is `    // FIXME(@alice): ` (RFC-016 rule 3).
6. The limit, and there is one on purpose: `TODO (@alice): ` (space before the
   parenthesis), `TODO(): ` and `TODO(@alice: ` are **not** a todo's word. A space means the
   word is over, a parenthesis that closes on nothing holds nothing to read or carry, and one
   that never closes is not a pair. Not every spelling can be supported, and a rule with no
   edge is a rule that guesses.
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

**Reading a bare word in the parentheses as a person** — `TODO(alice): ` is Alice. It is one
less character for the writer and one indistinguishable case for the reader, and the character
is not being saved: the body already writes `@alice`, so the version without the sigil is a
second spelling nobody needs. Worse, `TODO(A): ` would be a person called `A` sitting exactly
where a priority is written in every other todo. The word is carried and not read, which costs
the note nothing.

**Refusing the line when the parenthesis says nothing.** It was the first shape of this rule —
`TODO(bob): x` was not a todo — and it made a note invisible for a reason the note does not
mention. Dropping a todo is a bigger mistake than carrying a word nobody reads.

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
- **`TODO(bob): ` and `TODO(A): ` are todos with no assignee and no tag.** The parenthesis is
  in the lead, unread and unrewritten; RFC-019's table lists them, with `TODO(@bob): ` and
  `TODO(#home): ` beside them.
- **Three spellings are still not todos**, and the limit is stated rather than discovered:
  `TODO (@alice): `, `TODO(): ` and `TODO(@alice: `. A codebase that writes them will not see
  those notes — the one accepted cost this rule has, and RFC-019's table carries it.
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
  `a_parenthesis_that_is_not_a_person_or_a_tag_is_still_a_todo`,
  `a_parenthesis_that_does_not_touch_the_word_is_not_a_word`, and the amended
  `the_rule_is_narrow_and_its_misses_are_known` (four hits and three misses for this rule).
- `lib/src/ops/export.rs` — `a_person_named_before_the_colon_is_one_of_the_assignees`, and
  the copy round trip that still carries a tag, a person and a priority.
- `use-cases/tui/parentheses-name-a-person-or-a-tag` — the whole thing through a real pty:
  `sigil.rs  0/3  #2  @2` in the file panel with `#102  0/1` and `@alice  0/2` as
  collections, the bare-word line listed as a todo with nobody on it, and the space-before
  line on screen but not counted.

## Open questions

- **A collection for a ticket number.** `#102` is a tag and therefore a collection, so this
  is already answered — it is `/102`, in the tags of the panel header. Whether a number tag
  should be *shown* differently is the same question as below, and nothing filters on it.
- **Severity on the screen.** A `FIXME:` reads as one because the word is still in the line;
  nothing counts or folds by it (RFC-020's open question, unchanged).
- **Two people or two tags before the colon.** `TODO(@alice,@bob): ` is not read as two
  people: the parenthesis holds one token or nothing, so a comma list is carried as text and
  nobody is recognised. One value is the shape every tracker uses, and a list would need its
  own rule about separators.
- **`TODO (@alice):` with a space.** Out of scope, on purpose: the word has ended. A rule that
  reads past the space has to decide where the word stops, and that is the guessing this
  whole RFC is about avoiding.
