# Changelog

What changed in `lazytodo`, the terminal client. This package has its own version
and its own tag (`tui-v<version>`), independent of the other packages here.

## 0.1.0

The first release.

### Reading

- A folder or a single file: a folder opens every todo file in the tree, a file
  opens on its own.
- Todo files by the convention that names them, and todo comments inside source
  files, from a line comment to a block one.
- OPS documents, read and written, so a todo can leave the markdown world entirely.

### The screen

- Two layouts: a files panel beside a todos panel in a folder session, and the file
  alone when one file is opened.
- A file panel above a collection panel: every tag and every person gathered across
  the tree, with the document each one came from.
- Panels titled with their key, so the number on a border is the key that reaches
  it.
- Modals for adding, renaming, moving and searching, one at a time.
- Search across the tree, with the match count on the title it lands in.
- Notices in the header, and every answer in the status line rather than a popup.
- The screen redraws when the file changes underneath, and says so rather than
  overwriting.

### Editing

- The line grammar shared with `aricma-todo`: the checkbox, the priority letter,
  tags, people, headings and prose.
- Checking, unchecking, adding, deleting and moving a todo, with the cursor staying
  where the eye is.
- History: the last checks and unchecked boxes, drawn as a graph of what moved, and
  undone one at a time.
- Copy and paste of a todo, between files as well as inside one.
- Making and deleting todo files from the panel, with the name typed where it would
  appear.

### Keeping it honest

- Bounded walking: the depth, the number of files and the size of a file all stop
  at a limit that is said out loud, never a quiet truncation.
- A file too long to read is listed and marked, not opened.
- The same markdown rules the pipeline uses, so the two clients cannot disagree
  about what a line means.
