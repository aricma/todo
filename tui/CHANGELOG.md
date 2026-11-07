# Changelog

What changed in the `lazytodo` screen. This crate has its own version and its own tag (`tui-v<version>`),
independent of the other packages in this repository.

## 0.1.0

### Features

- feat(todo): a person in the word, a ticket in the body
- feat(lazytodo): a history entry is one line, and h wraps it
- feat(lazytodo): a single file scrolls, and a session needs something to read
- feat(lazytodo): a band marks the text, and stops before the bar
- feat(lazytodo): the history band is on the label, not the dot
- feat(lazytodo): number the panels, and the number is the key
- feat(lazytodo): a thumb when a panel has more rows than room
- feat(lazytodo): a history row carries a dot, not a checkbox
- feat(lazytodo): a line above a comment todo, three below
- feat(lazytodo): an inline file takes only the keys it can
- feat(todo): a copied todo is an OPS document, pasted as lines
- feat(lazytodo): a new todo is the row it is typed into
- feat(todo): a move past a heading joins that chapter's todos
- feat(lazytodo): dim the screen behind a modal
- feat(todo): a move crosses a heading instead of stopping at it
- feat(lazytodo): number the lines of the file view too
- feat(lazytodo): keep the split when a single file is open
- feat(lazytodo): the TUI, rebuilt as state, ui, terminal and helpers
- feat(lazytodo): add the markdown todo client

### Fixes

- fix(tui): the header shows its path with a slash too
- fix(tui): show a relative path with a slash on every machine
- fix(todo): an unknown parenthesis is text, not a refusal
- fix(todo): a number in a hash is a tag, and parens need a sigil
- fix(lazytodo): space undoes a write, and h comes last and says hide
- fix(lazytodo): the arrows cross in a single-file session too
- fix(lazytodo): draw the thumb after the rows, so no band cuts it
- fix(lazytodo): the thumb spans the track and never takes the band
- fix(lazytodo): the window holds, and the fold at the top is drawn
- fix(todo): a todo needs capitals, a marker and no quote
- fix(lazytodo): the arrows are nav, not a todo
- fix(lazytodo): keep the add field, with the empty row behind it
- fix(lazytodo): keep the dependency lanes on a wrapped history row
- fix(lazytodo): call the enter key submit, not commit
- fix(lazytodo): put copied text on a clipboard that gets it

### Refactors

- refactor(lazytodo): the crate is lazytodo, testkit sits beside it, and a snapshot does not name the machine
- refactor(todo): comments first, then four words in them
- refactor(lazytodo): a single file is the same window, one panel fewer
- refactor(lazytodo): a comment todo file, not an inline one
- refactor(lazytodo): every panel says nav for the arrows
- refactor(lazytodo): drop the rule under the new todo field

### Documentation

- docs(lazytodo): the manual is the tui's README, with a man page as the next step
- docs(lazytodo): the three new todos, with where the code is
- docs(lazytodo): a comment todo is four words, not one
- docs(lazytodo): four rfcs for the todos in the code
- docs(lazytodo): an rfc for the checkbox, and what a bullet is not
- docs(lazytodo): seven more rfcs, from collections to the keys
- docs(lazytodo): a glossary of the panels, and the first four RFCs

### Tests

- test(cases): keep a main where the pty harness is compiled out
- test(cases): the pty screen suite is unix-only until conpty works
- test(cases): say what the pty child was doing when the screen is blank
- test(lazytodo): a use case for reading a todo in code

### Style

- style(todo): the crates are rustfmt-clean

### Chores

- chore(todos): windows is in, and the conpty gap is written down
- chore(license): MIT for aricma-todo, closed for the monorepo
- chore(lazytodo): tick the three todos this pass closed
- chore(todos): assign todos to @aricma and @kevin for a tui test
- chore(lazytodo): the branch's own todos, ticked
