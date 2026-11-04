//! The search popover: a query, the todos it found, and the one selected.
//!
//! The query survives closing, so opening it again is one keystroke. Clearing is
//! `ctrl+u` rather than `x`: this is a text field, and a key that erases what you
//! typed makes it impossible to search for "xfce".

use aricma_todo_lib::discover::Entry;
use aricma_todo_lib::search::{self, Hit};

use super::message::Message;
use super::prompt::Prompt;
use crate::text;

/// The whole state of the popover.
#[derive(Debug, Default)]
pub struct Search {
    pub query: Prompt,
    pub hits: Vec<Hit>,
    pub selected: usize,
    /// What the last search has to say: nothing found, or why it could not run.
    pub message: Option<Message>,
}

impl Search {
    /// Re-run the query over every file's todos. The files are asked for rather
    /// than kept: the app's list is what is on disk right now, and a copy here
    /// would be one more thing to remember to update. A file that cannot be read
    /// says why under the query.
    pub fn refresh(&mut self, files: &[Entry]) {
        match search::find(files, self.query.text()) {
            Ok(hits) => {
                // an empty result is information, not a failure: nothing is wrong
                // with the query, there is simply nothing that matches it
                self.message = hits.is_empty().then(|| Message::info(text::NO_MATCH));
                self.hits = hits;
            }
            Err(error) => {
                self.message = Some(Message::failed(error.to_string()));
                self.hits.clear();
            }
        }
        self.selected = self.selected.min(self.hits.len().saturating_sub(1));
    }

    pub fn select(&mut self, delta: isize) {
        let last = self.hits.len().saturating_sub(1);
        self.selected = if delta.is_negative() {
            self.selected.saturating_sub(delta.unsigned_abs())
        } else {
            (self.selected + delta.unsigned_abs()).min(last)
        };
    }

    /// Empty the query, and the results with it.
    pub fn clear(&mut self, files: &[Entry]) {
        self.query = Prompt::new("");
        self.selected = 0;
        self.refresh(files);
    }

    pub fn current(&self) -> Option<&Hit> {
        self.hits.get(self.selected)
    }

    /// How tall the popover has to be to show what it has: the query row, the
    /// message if there is one, and one row per hit.
    pub fn height(&self) -> u16 {
        let message = u16::from(self.message.is_some());
        u16::try_from(self.hits.len())
            .unwrap_or(u16::MAX)
            .saturating_add(1 + message)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::Search;
    use crate::testing::app;

    /// The files a search runs over: the app's own list, asked for each time.
    fn files(root: &std::path::Path) -> Vec<aricma_todo_lib::discover::Entry> {
        aricma_todo_lib::discover::scan(root).unwrap().entries
    }

    fn typed(search: &mut Search, files: &[aricma_todo_lib::discover::Entry], text: &str) {
        for character in text.chars() {
            search.query.insert(character);
            search.refresh(files);
        }
    }

    #[test]
    fn typing_narrows_the_results_to_the_best_first() {
        let (_tmp, app) = app(&[(
            "TODOs.md",
            "# milk\n\n- [ ] email the landlord\n- [ ] buy milk\n",
        )]);
        let files = files(&app.root);
        let mut search = Search::default();
        search.refresh(&files);
        assert_eq!(search.hits.len(), 2, "an empty query lists every todo");

        typed(&mut search, &files, "ml");
        let bodies: Vec<&str> = search.hits.iter().map(|hit| hit.body.as_str()).collect();
        assert_eq!(bodies, ["buy milk", "email the landlord"]);
    }

    #[test]
    fn the_query_and_its_hits_are_kept_until_they_are_cleared() {
        let (_tmp, app) = app(&[("TODOs.md", "- [ ] buy milk\n- [ ] pay rent\n")]);
        let files = files(&app.root);
        let mut search = Search::default();
        search.refresh(&files);
        typed(&mut search, &files, "milk");
        assert_eq!(search.hits.len(), 1);

        // closing the popover is a mode, not a field of this struct: nothing here
        // is thrown away when the keyboard goes back to a panel
        search.refresh(&files);
        assert_eq!(search.query.text(), "milk", "the query is still there");
        assert_eq!(search.hits.len(), 1, "and so are its results");
    }

    #[test]
    fn clear_empties_the_query_and_the_selection() {
        let (_tmp, app) = app(&[("TODOs.md", "- [ ] buy milk\n- [ ] pay rent\n")]);
        let files = files(&app.root);
        let mut search = Search::default();
        search.refresh(&files);
        typed(&mut search, &files, "milk");
        search.select(1);

        search.clear(&files);
        assert_eq!(search.query.text(), "");
        assert_eq!(search.selected, 0);
        assert_eq!(search.hits.len(), 2, "everything is back");
    }

    #[test]
    fn selecting_stops_at_the_ends() {
        let (_tmp, app) = app(&[("TODOs.md", "- [ ] a\n- [ ] b\n")]);
        let files = files(&app.root);
        let mut search = Search::default();
        search.refresh(&files);
        search.select(1);
        assert_eq!(search.selected, 1);
        search.select(1);
        assert_eq!(search.selected, 1, "clamped at the last hit");
        search.select(-5);
        assert_eq!(search.selected, 0);
    }

    #[test]
    fn the_popover_is_as_tall_as_its_query_and_its_hits() {
        let (_tmp, app) = app(&[("TODOs.md", "- [ ] buy milk\n- [ ] pay rent\n")]);
        let files = files(&app.root);
        let mut search = Search::default();
        search.refresh(&files);
        // two hits and the query row
        assert_eq!(search.height(), 3);

        typed(&mut search, &files, "zzz");
        // nothing found: the query row and the line that says so
        assert_eq!(search.height(), 2, "{:?}", search.message);
    }

    #[test]
    fn a_hit_names_the_file_and_the_line_to_jump_to() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("TODOs.md");
        fs::write(&path, "- [ ] buy milk\n").unwrap();
        let files = aricma_todo_lib::discover::scan(&path).unwrap().entries;
        let mut search = Search::default();
        search.refresh(&files);
        let hit = search.current().expect("the only hit");
        assert_eq!(hit.line, 0);
        assert_eq!(hit.path, path);
    }
}
