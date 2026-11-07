# Changelog

What changed in the markdown rules the two clients share. This crate has its own version and its own tag (`lib-v<version>`),
independent of the other packages in this repository.

## 0.1.0

### Features

- feat(todo): a person in the word, a ticket in the body
- feat(lazytodo): an inline file takes only the keys it can
- feat(todo): the tree is what git does not ignore
- feat(todo): a TODO: comment is a todo, in any code file
- feat(todo): a copied todo is an OPS document, pasted as lines
- feat(lazytodo): a new todo is the row it is typed into
- feat(todo): a move past a heading joins that chapter's todos
- feat(todo): a move crosses a heading instead of stopping at it
- feat(todo): collections, fuzzy search and scan limits
- feat(lazytodo): add the markdown todo client
- feat(todo): read and write OPS with the reference ops-lib
- feat(todo): the local dev backends as default sources
- feat(place): open the store through its resolved place, and mirror it
- feat(place): resolve which place the local store lives in
- feat(todo): a Field's value can be fixed, not only defaulted
- feat(ftp): a lib-only FTP transport, proven against real vsftpd
- feat(remote): address an S3 bucket as a subdomain too
- feat(remote): give every remote an aricma-todo namespace folder
- feat(remote): s3 transport with sigv4 signing
- feat(remote): webdav transport
- feat(db): one store, OPFS when available, in memory otherwise
- feat(model): context-gated storage messages and config fields
- feat(sync): Google Drive data source
- feat(source): replace Provider::caveat with a tone-typed Message
- feat(source): display the local provider as Browser
- feat(todo): report import and export where the work happens
- feat(todo): make the data source a provider choice, not a path
- feat(todo): recurring instance chains in the lib — complete_occurrence stamps recurring_id + instance_seq (ADR-0013)
- feat(todo): reschedule due date with recurrence intact
- feat(todo): smart date picker component with presets, month nav, and postpone
- feat(search): highlight matching title substrings in inbox results
- feat(ops): OPS-compatible csv/markdown/jsonl, ops module split
- feat(theme): Theme enum with per-branch data, app constants

### Fixes

- fix(todo): an unknown parenthesis is text, not a refusal
- fix(todo): a number in a hash is a tag, and parens need a sigil
- fix(todo): a todo needs capitals, a marker and no quote
- fix(todo): list a todos file that holds nothing but headings
- fix(sync): refuse a partial database before SQLite sees it
- fix(drive): request the `id` the version guard parses
- fix(db): report missing cross-origin isolation, not 'no such vfs'
- fix(todo): instance_seq stored as a JSON number, read as one (ADR-0013)

### Refactors

- refactor(todo): comments first, then four words in them
- refactor(lazytodo): a comment todo file, not an inline one
- refactor(todo): nest the SPA under web
- refactor(todo): keep one path-style S3 config with a region
- refactor(model): name the domain DataSource, not Storage
- refactor(model): split storage source data
- refactor(sync): one transport enum, Drive as its first variant
- refactor(source): look up the provider by id
- refactor(todo): name the browser layer side_effects, and put the theme in the lib
- refactor(state): pure AppState transitions + add_todo action in lib states/
- refactor(parse): move Parsed → Task pipeline into lib with injected tz conversion
- refactor: split src/ into lib/ core and app/ bin

### Documentation

- docs(plans): plan the S3 addressing, icons and DataSource rename
- docs(todo): ADR-0013 recurring instance chains supersedes ADR-0006 roll-forward

### Tests

- test(scan): compare relative paths with one separator on every machine

### Style

- style(todo): the crates are rustfmt-clean

### Chores

- chore(license): MIT for aricma-todo, closed for the monorepo
