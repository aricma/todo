# RFC-021: Who it is for, and which ticket

- **Date:** 2026-11-06
- **Related:** [RFC-013](RFC-013-a-todo-is-a-checkbox.md), [RFC-014](RFC-014-a-copied-todo-is-a-document.md), [RFC-016](RFC-016-a-comment-todo.md), [RFC-019](RFC-019-not-every-todo-is-a-todo.md), [RFC-020](RFC-020-four-words-and-a-marker-per-language.md)

## Context

RFC-020 refused two things, and refused them for one reason: `TODO(alice):` and a `#102`
ticket would each be a *second way to say something the model already had a way to say* —
`@alice` for a person, `#home` for a `#` token — and two spellings of one field is how two
parsers start disagreeing about the same line.

The reasoning was sound about **storage** and wrong about **syntax**. A repository writes
`// TODO(alice): #102 the retry is wrong` because that is the convention of the tracker its
team uses, and not reading it is not the same as agreeing with it: the note is there, in the
code, and the tool that refuses to see it is simply less useful than the one that does.

So the refusal is reversed in the way that keeps the original worry true:

- the parentheses do **not** become a second field — they are read into the one list of
  people that already exists;
- the ticket **does** become its own field, because it was silently masquerading as a tag,
  and a collection called `102` is not a collection anybody asked for.

## Behaviour

### Who it is for

1. A severity word **MAY** be followed by an assignee in parentheses and then its colon:
   `TODO(alice): `, `FIXME(dave-b): `. The parentheses **MUST** hold one word — letters,
   digits, `_` or `-` — and the `)` **MUST** be followed immediately by `:`.
2. The person in the parentheses **MUST** be normalised into `task.assignees`, the same list
   `@alice` in the body goes into, and **MUST NOT** become a field of its own. One line has
   one set of people on it, however the line spells them.
3. They are **MUST** be first in that list, in written order, and a person named both ways
   **MUST** appear once: `TODO(alice): ship it @alice` names one person.
4. The parentheses are an assignee and **MUST NEVER** be read as a priority. `// TODO(A):
   pay rent` names someone called `A`; a priority is written **after** the colon, where the
   body is (RFC-016 rule 4), so `// TODO(alice): (A) pay rent` is a person and a priority.
   This is the collision RFC-020 was afraid of, and it is settled by position rather than
   by shape — a single letter is still a word.
5. The text of the parentheses **MUST** be preserved in the line's lead and the tool **MUST
   NOT** rewrite it: `task.lead()` is `    // FIXME(alice): ` (RFC-016 rule 3).
6. `TODO (bob): `, `TODO(): ` and `TODO(bob: ` are **not** a todo's word: a space means the
   word is over, a parenthesis that closes on nothing names nobody, and one that never
   closes is not a pair. RFC-019's table carries all three as misses.

### Which ticket

7. A `#` token **MUST** be read as a **ticket** when its word is either all digits
   (`#102`, `#2024`) or capitals, one dash, digits (`#PROJ-404`, `#JIRA-7`, `#A-1`).
   Otherwise it is a **tag**, as before (`#home`, `#worst-bug-of-the-century`, `#2fa`).
8. The **first** ticket-shaped token **MUST** be `task.ticket`, and a second one **MUST**
   stay a tag: a line that names two tickets is about the one it names first, and the
   second is a label like any other. A ticket **MUST NOT** also be a tag, and **MUST NOT**
   appear in `parts.title`.
9. The ticket **MUST** be read by the same tokenizer as the tags, so a markdown todo and a
   comment todo agree: `- [ ] #102 the retry` and `// TODO: #102 the retry` are the same
   task with a different source. A rule that applied to one source and not the other would
   be the second parser this repository keeps refusing to write.
10. In an OPS document the ticket **MUST** travel under `aricma_ticket` — the spec has no
    common key for a ticket, and a tool's own data goes under a key prefixed with the tool's
    name (OPS RFC-019). The export **MUST** read the task rather than parsing the body
    again, or the person in the parentheses — who lives in the part of the line the tool
    does not own — would be exported by nobody.
11. A ticket **MUST** be drawn in a colour of its own (blue), because it is the one thing
    in a body that points somewhere else.

## The tables

Asserted by `a_number_is_a_ticket`, `an_upper_case_project_key_and_a_number_is_a_ticket`
and `a_tag_is_not_a_ticket` in `lib/src/model/ticket.rs`.

| token | read as | why |
| --- | --- | --- |
| `#102`, `#1`, `#2024`, `#0` | ticket | all digits |
| `#PROJ-404`, `#JIRA-7`, `#AB-12`, `#A-1` | ticket | capitals, one dash, digits |
| `#home`, `#infra` | tag | a word that is not a number |
| `#worst-bug-of-the-century` | tag | dashes and letters |
| `#2fa` | tag | digits then letters: a hash, not a number |
| `#proj-404`, `#a-1` | tag | the key is lowercase: a project key is written in capitals |
| `#PROJ-2026-7`, `#PROJ-1-2` | tag | one dash only, so a long key is not a ticket |
| `#`, `#two words`, `102` | nothing | not a `#` token at all |

## Why not the alternative

**A `Todo.assignee` field and a `Todo.ticket` field, as the first draft of this had.** Two
fields for people, and a second value to keep in step with `assignees` on every edit, copy
and check. The parentheses are a spelling, not a storage, so they are normalised at parse
time and forgotten.

**Reading the parentheses as a priority when they hold one letter.** It looks convenient —
`TODO(A):` reads like an important note — and it makes the same three characters mean two
things depending on how many letters are inside. Position already decides: what is after the
colon is the body, and the body is where a priority lives.

**Allowing a space before the parentheses** — `TODO (bob): `. It reads better in prose and it
makes the word's end ambiguous: `TODO (not a person) x` would have to be resolved by trying,
and a rule that has to try is a rule that guesses twice.

**A ticket without the `#`,** as `TODO: PROJ-404 the retry is wrong`. The `#` is what says
"this token is not a word of the title", and without it `PROJ-404` is a hyphenated word in a
sentence. The token is optional anyway: a note without a ticket is the normal case.

**Lower-case project keys as tickets.** `#a-1` could be a ticket and `#2fa` could not, and
the only difference a reader can see is the case. Capitals only, one dash only — a shape
simple enough to state in a line, which is the whole test of a rule here.

## Consequences

- **A ticket-shaped tag becomes a ticket.** `#2024` was a tag and is now a ticket, so the
  `2024` collection is gone. A tag named for a year is unusual enough to pay for a rule with
  no exception; the token still reads the same in the line, and it still exports.
- **The same reading applies to documents.** `- [ ] #102 fix it` in a `TODOs.md` reports a
  ticket now. One rule, one tokenizer, both sources (rule 9).
- **`TODO(A):` is a person.** A codebase that writes a priority before the colon gets it as
  an assignee — the priority is still read where the grammar puts it.
- **`#PROJ-2026-7` is a tag.** A long project key with a year in it is not a ticket; widening
  the shape is an open question below.
- **A ticket stays in the title of an OPS document.** `title` is the line's text without the
  priority token (RFC-014), and tags already stay there, so the ticket does too; the
  `aricma_ticket` key is the structured copy.
- **RFC-019's table grows two rows and loses one.** `// TODO(bob): pay rent` is listed, and
  the malformed parentheses join the misses.

## How it is proven

- `lib/src/doc/parse.rs` — `a_word_may_name_who_it_is_for`,
  `the_people_in_the_word_and_in_the_body_are_one_list`,
  `the_parentheses_are_an_assignee_and_never_a_priority`, `a_ticket_number_is_not_a_tag`,
  and the amended `the_rule_is_narrow_and_its_misses_are_known`.
- `lib/src/model/ticket.rs` — the three shape tests above.
- `lib/src/ops/export.rs` — `a_ticket_travels_under_a_namespaced_key`, and the old copy test
  that still round-trips a line with a tag, an assignee and a priority.
- `use-cases/tui/assignee-and-ticket-in-a-comment` — the whole thing through a real pty:
  `ticket.rs  0/2  @2` in the file panel, `@alice  0/2` and `@bob  0/1` as collections, and
  `0/2  @2` in the panel footer with no tag count, because `#102` is not a tag.

## Open questions

- **A ticket collection?** `#102` is a ticket and not a tag, so it is not a collection and
  `#` in the header does not count it. Whether a person wants to see "every todo about
  PROJ-404" — the collection view for tickets — is a design question with no evidence yet.
- **A long project key.** `#PROJ-2026-7` is a tag today. Allowing digits inside the key
  would read it, and would also read `#2fa-1` as a ticket, which is a worse trade.
- **Severity and assignee in a panel header.** The count line says `#tags` and `@people`;
  it does not say how many `FIXME`s a file holds. Nothing filters on `severity` yet
  (RFC-020's own open question), and a count is only useful once something can be done
  with it.
- **Two people or more before the colon.** `TODO(alice,bob):` is not read. One name is the
  shape every tracker uses for "the owner", and a comma list would need its own rule about
  spaces.
