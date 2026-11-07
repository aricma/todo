# Changelog

What changed in `aricma-todo-lib`, the markdown todo rules the two clients share.
This package has its own version and its own tag (`lib-v<version>`), independent of
the other packages here.

## 0.1.0

The first release. Everything below is new, so nothing here is a fix.

### The rules

- A todo is a markdown line: checkbox, priority letter, body, tags and people, and
  every one of those is read and written without touching the words around it.
- Headings, prose, code fences and front matter are left exactly as they were,
  including line endings, so a rewrite is a rewrite of the todos only.

### Finding todos

- Todo files by the convention that names them, or every markdown file when asked.
- Todo comments inside source files, per language, from a line comment to a block
  one.
- A bounded walk: depth, file count and file size, each reported when it stops a
  file instead of hiding it.

### Working with them

- `check` reports what breaks the rules and writes nothing.
- `format` fixes the checkbox case and the priority letter case.
- Collections: every tag and every person, gathered across a set of files, each
  pointing back at the document it came from.
- OPS export and import, so the same todos can be read and written as an OPS
  document.
