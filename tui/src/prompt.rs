//! One editable line, with a cursor. The only text field in the tool.

/// What the keyboard is doing right now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Moving the cursor and acting on the selection.
    Normal,
    /// Typing a new task, with the text so far.
    Adding(Prompt),
    /// Editing the body of the selected task.
    Editing(Prompt),
    /// Waiting for `y` or `n` after a delete.
    Confirming,
}

/// The text being typed and where the cursor sits, counted in characters so a
/// multi-byte body cannot split.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prompt {
    text: String,
    cursor: usize,
}

impl Prompt {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            cursor: text.chars().count(),
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn insert(&mut self, character: char) {
        let at = self.offset();
        self.text.insert(at, character);
        self.cursor += 1;
    }

    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.cursor -= 1;
        let at = self.offset();
        self.text.remove(at);
    }

    pub fn delete(&mut self) {
        if self.cursor >= self.text.chars().count() {
            return;
        }
        let at = self.offset();
        self.text.remove(at);
    }

    pub fn left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn right(&mut self) {
        self.cursor = (self.cursor + 1).min(self.text.chars().count());
    }

    pub fn home(&mut self) {
        self.cursor = 0;
    }

    pub fn end(&mut self) {
        self.cursor = self.text.chars().count();
    }

    /// The byte offset of the cursor.
    fn offset(&self) -> usize {
        self.text
            .char_indices()
            .nth(self.cursor)
            .map_or(self.text.len(), |(index, _)| index)
    }
}

#[cfg(test)]
mod tests {
    use super::Prompt;

    #[test]
    fn starts_with_the_cursor_at_the_end() {
        let prompt = Prompt::new("abc");
        assert_eq!(prompt.text(), "abc");
        assert_eq!(prompt.cursor(), 3);
    }

    #[test]
    fn inserts_and_deletes_at_the_cursor() {
        let mut prompt = Prompt::new("ac");
        prompt.left();
        prompt.insert('b');
        assert_eq!(prompt.text(), "abc");
        prompt.backspace();
        assert_eq!(prompt.text(), "ac");
        prompt.delete();
        assert_eq!(prompt.text(), "a");
    }

    #[test]
    fn edges_are_safe() {
        let mut prompt = Prompt::new("");
        prompt.backspace();
        prompt.delete();
        prompt.left();
        assert_eq!(prompt.text(), "");
        prompt.home();
        prompt.insert('x');
        prompt.end();
        prompt.right();
        assert_eq!(prompt.cursor(), 1);
    }

    #[test]
    fn multi_byte_text_is_not_split() {
        let mut prompt = Prompt::new("café");
        prompt.backspace();
        assert_eq!(prompt.text(), "caf");
        prompt.insert('é');
        assert_eq!(prompt.text(), "café");
        prompt.home();
        prompt.insert('x');
        assert_eq!(prompt.text(), "xcafé");
    }
}
