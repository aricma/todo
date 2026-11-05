# aricma-todo-tui todos

- [x] (A) the struct Search should not keep files: Vec<Entry> because of: "Files to search: the scan's entries, kept so a reopen needs no rescan.", we should rather just fetch the files fresh
- [x] the wrap inside the new todo modal works but I do not see the cursor advancing into the second or third row
- [x] (A) on new file we need to auto create the h1 as "# <parent-folder-name> todos" which we basically always do anyway
- [x] (B) use ──╮  │ ╰─── chars for the histroy dependency they look much better as connections
- [x] (C) we need to have the outline panel always present so we can add a 'n' keybinding to add headings to files
- [x] (D) opt+up/down still does not work — investigated: every modified arrow is already bound (`alt`, `meta`, `shift`, `ctrl`, `super`), so it is the terminal. macOS Terminal.app with "use option as meta" sends `ESC ESC [ A` for opt+↑ and crossterm reads the doubled ESC as the Esc key, leaving `[` and `A` as letters — undecodable. How to fix properly: push `PushKeyboardEnhancementFlags` on startup when `supports_keyboard_enhancement()`, which makes capable terminals (kitty, wezterm, foot, Ghostty) send `CSI 1;3A`. Until then: `K`/`J`, or `ctrl+↑`/`ctrl+↓` (iTerm2 sends `CSI 1;5A`)
- [x] we need to add a way to copy a selected file path in the filespanel and a todo inside the todos panel
- [x] the new todo modal is now too small ... if we can only deal in 3 or 1 column we should use the 3 but add a black line for the text input field so it does not look so slim
- [x] (B) the new todo inpu should be less width but we should wrap
- [x] cmd+f for search is taken so we should just do 'f'
- [x] (A) add more stats for how many tags and assignees are in each file in the files panel make the stats color coded as we have them in the todos panel as well
- [x] (D) add a conflict resolution to the todos panel
- [x] (B) increase the depth we can handle to 10 levels
- [x] (A) add the same stats for tags and assignees to the bottom right of the todos panel next to the todo numbers @aricma @kevin
- [x] (B) allow the history items to wrap
- [x] (B) allow headings in outline to wrap
- [x] (C) rename outline to index
- [x] (C) add a 'c' keybinding for the todos panel and files panel so we can copy files and todos to the clipboard with a single keystroke
- [x] (D) how do we give feedback for copied todos and files
- [x] (A) add a header line above the panels, a single line that carries the name of this app "aricma-todo .  $PWD"
- [x] "─── 16/18  #1  @1 ╯" this 16/18 stat needs to be the same color as the border when the panel is active, right now the stat gets darker when the panel is in focus
- [x] we should allow not only TODOs.md files but also Tasks.md files in the files panel — already so: `is_candidate` lowercases the name and accepts `task*`, with `accepts_todo_and_task_prefixes_in_any_case` for `Tasks.md` and `tasks-2026.md`
- [x] collections in the files panel can not be deleted — right: deleting one would mean stripping `#tag` from every line that names it, across files, which this tool does not do. So there is no key for it, and now it is not offered either: the status bar follows the **row**, not the panel, so a collection row shows `↑/↓ collection` and no `n`/`d`, and the keys page says "a file row"


## open, in the order to do them

Priority first, then the shortest way to get it. (Nothing open: the rest of this file is
the record of how each one was decided.)

- [x] (A) the stats for collections in the files panel shoul be the same as for the  other todo  files done/total not just total
- [x] (A) when we highlight collections in the files panel we can not create a new file based on that collection this keybinding should not be possible we have to make keybinding hints visibile for the files panel not only based on the active panel but based on the highlighted item
- [x] (B) tags and assignees aka todo collections in the files panel need to  be sorted by amount of todal todos since we are not looking for them by name and rather by size, to find a tag by name we will use the search
- [x] (B) history actions should have better stadardised names that do not differ so much in length like 'add' and 'priority B' rather 'new todo' and  'set prio B'
- [ ] (C) add shift+up/down to also move to the first and last item in every panel next to g/G — bound, but macOS Terminal.app drops the modifier on arrows, so `shift+↑` arrives as a plain `↑`; `home`/`end` were added as a spelling no terminal can eat. Left open until it is tried in a terminal that reports modifiers (kitty/wezterm/foot/Ghostty/iTerm2)
- [x] (C) add opt+up/down as bindings to also switch left sidebar panels — moving a todo is `ctrl+↑/↓` now (`K`/`J` stay)
- [x] (D) add an 'h' keybinding for the files panel so that we can unhide/hide all other *.md files in the tree since any of them  could have todos in them, this is for teams the do not stic to the TODOs.md and Tasks.md name conwentions and want to just have a look around but these other md files are not included in the search and are hidden by default
- [x] file panel files and collections should both support a 'c' binding to copy the litteral string like  the path tag or assignment
- [x] (B) add line numbers to the todos panel — the todos panel had them all along; the panel that did not was the **file view**, which now numbers every line it draws, prose and headings included, with the file's own line number (RFC-002)
- [x] (A) the tui in single file mode should not collapse and hide the files panel but keep the left right split — one file hides the files panel, not the split: the index, todos and history keep the right-hand column and its width
- [x] (B) empty files are not visible in the files panel but they should — a candidate file is listed when it holds a todo, a line that meant to be one, or nothing but headings: a file made for the first todo is a place to go (a readme with prose and no todos stays out)
- [ ] (D) we should be dealing with 'TODO:' comments in any file in the tree
- [ ] (C) we should copy todos as OPS so we can copy paste them between files
- [ ] (D) we need to fix the new todo input wrap ... it is a weird user experiance + the spaces are trimmed from the state before passed down to the view again with leads to a weird experinace where spaces are not visible until wrapped by another word
- [ ] (D) we need to add hints for todos to be formatted correctly and the files to be formatted correctly and auto fix functions so the user can just accept them
- [ ] (D) add auto fix hints for the headdings panel, if an h1 or h2 for 'Done ✅' is missing we are adding a hint that can be approved by navigating to it and press Y  for now we do not allow to dismiss any hint since we do not have state other then the pure files BUT we do allow to press h to hide or unhide the hints
