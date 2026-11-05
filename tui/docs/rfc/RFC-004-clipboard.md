# RFC-004: The clipboard and its feedback

- **Date:** 2026-10-02
- **Related:** [RFC-001](RFC-001-panels-and-their-names.md), `tui/src/terminal/perform.rs`

## Context

A terminal program on the far side of a pty cannot reach the system clipboard
directly. The established way is the terminal's own escape sequence — **OSC 52** —
which a terminal that implements it turns into a clipboard write, and which a
terminal that does not implement it ignores **silently**. There is no reply to
listen for: the sequence is write-only.

That silence caused a shipped bug. `c` printed `copied - [ ] pay rent` in green, in
the header, on macOS **Terminal.app** — which does not implement OSC 52 — and the
clipboard still held whatever it held before. The message was telling the truth
about the program and a lie about the person's clipboard.

## Behaviour

1. `c` **MUST** put on the clipboard, by the panel the cursor is in: the file's path
   in the files panel, the collection's own word (`#home`, `@sam`) on a collection
   row, a heading's text in the index panel, and — in the todos panel, the file view
   or a collection — an **OPS document** of the selected todo. A todo is data, so it
   travels as data; RFC-014 says what that document is and how it comes back.
2. A copy **MUST NOT** reformat what it copies: a path is a path, a heading is its
   text, a collection is its word. A todo is the exception the structure buys: it is
   not the file's line, because a line is a rendering of the todo and the document is
   the todo.
3. The program **MUST** attempt the OSC 52 sequence on every copy — it is the only
   clipboard that reaches a screen over ssh.
4. On a local macOS session (`SSH_CONNECTION` and `SSH_TTY` both unset) it **MUST**
   also write through `pbcopy`, because the terminal in front of the person may be one
   that ignores OSC 52. Over ssh it **MUST NOT**: `pbcopy` there fills the far
   machine's clipboard, which is not the one being pasted into.
5. Feedback **MUST** be a success message in the header — `copied <what>` — at the
   right-hand end of the top line, truncated to half the row. It **MUST NOT** claim
   the text landed, because the program cannot know that.
6. A copy **MUST NOT** change the file, the history, or the cursor: it is the one key
   with no consequence on disk.
7. A panel where nothing is copyable (the history panel) **MUST** offer no `c` in the
   status bar.

## Why not the alternative

A clipboard crate (`arboard` and friends) was rejected: it links to the platform's
clipboard APIs, which is a dependency and a build problem on every platform, and it
does not work when the program is running on the far side of an ssh session — the
case OSC 52 exists for. `pbcopy` on macOS is a platform feature already installed,
one process per copy, and it fills the clipboard the person will actually paste from.

Telling the person up front that their terminal does not support OSC 52 was rejected:
there is no way to find out without a reply that never comes, so the honest options
are to write through a second path where one exists, and to word the message as what
was copied rather than where it went.

## Consequences

- Two paths mean two things to keep true, and the tests cover the part that can be
  tested: the local-versus-ssh decision, and that the helper actually feeds the text
  to `pbcopy` on stdin (with a stand-in `pbcopy` on `PATH`).
- Linux has no fallback yet: a terminal there that ignores OSC 52 is a copy that goes
  nowhere, and `wl-copy`/`xclip` are the obvious next two lines.
- The header carrying exactly one kind of message is what keeps a copy from pushing
  the keys out of the status bar.

## Open questions

- Is `wl-copy`/`xclip` worth the same treatment on Linux, or is OSC 52 good enough on
  the terminals Linux users actually run?
- Should a copy that is *known* to have gone nowhere (a capability probe, once a
  terminal ever answers one) say so, and where?
