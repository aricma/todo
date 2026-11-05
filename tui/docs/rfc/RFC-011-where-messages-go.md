# RFC-011: Where messages go

- **Date:** 2026-10-02
- **Related:** [RFC-001](RFC-001-panels-and-their-names.md), [RFC-004](RFC-004-clipboard.md), [RFC-008](RFC-008-history-and-undo.md)

## Context

Everything this program says to a person is either about a thing on screen or about
the thing that just happened to it. A single status line for all of it was the first
shape and it was wrong twice: it pushed the keys out of the bar (the keys are the
interface), and it put the explanation of a panel's problem far from the panel.

So a message is drawn **where its subject is**, and one line is kept free for the only
message whose subject is not on screen: a success.

## Behaviour

1. A **success** (`copied - [ ] pay rent`, `reloaded from disk`) **MUST** be drawn at the
   right-hand end of the **header**, truncated to half the row, and **MUST** be the only
   message the header ever carries.
2. A **failure** (a write that did not land) and a **refusal** (a rule that stops a key)
   **MUST** be drawn in the **status bar**, where the keys are — a failure takes the
   keys' place, because a key that will fail is not worth offering.
3. A message about a **panel** (a scan that stopped at its limit, a collection that could
   not be gathered) **MUST** be drawn in that panel's own footer, above its bottom
   border, and the panel **MUST** keep at least its minimum list height while it does.
4. A message about one **row** (history entry 3 cannot be redone) **MUST** be drawn on
   that row while the cursor is on it, derived from state rather than stored, so it
   cannot outlive its cause.
5. The **next key MUST** clear whatever message is on screen, except a message that is
   derived from state (rules 3 and 4), which stays while its cause does.
6. Tone **MUST** be one of three: success (green, header), warning (a rule explained:
   yellow), failure (red). A refusal **MUST NOT** be a failure: nothing went wrong.
7. A message **MUST** name the thing it is about in the words of the glossary, and
   **MUST NOT** name a key that would not work (RFC-009 rule 3).
8. An empty search result **MUST** be information in its own popover, never a failure:
   nothing is wrong with the query.
9. Every word **MUST** come from `text.rs`. A string literal in a panel is a bug, because
   the keys page, the status bar and the messages all have to read as one voice.

## Why not the alternative

**One log line at the bottom** was rejected: the status bar is where the keys live, and a
program whose bar is sometimes keys and sometimes a log makes both unreadable. It also
made "which panel is this about" a guess.

**A dismissable toast/overlay** was rejected: it covers the thing it is about, and it
needs a key to dismiss — which is a mode, in a program that already has enough of them.

**No success messages** was considered: they are the only feedback a copy has
(RFC-004). They are limited instead — one kind of message, one place, half a row.

## Consequences

- The header is a contract: it says what this is, where it is, and what just worked. A
  second message type there would break the reading of all three.
- A panel footer costs rows, so a panel with something to say is shorter than one
  without; the minimum list height is what keeps that from eating the list.
- Derived row messages mean the state does not grow a message field per entry that has
  to be cleared by someone remembering to clear it.

## Open questions

- Should a success that a later failure contradicts (`copied`, then a failed write) clear
  immediately, or wait for the next key like every other message?
- Should the header show a *count* of messages that did not fit (three files unreadable)
  rather than the first one?
