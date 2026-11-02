//! Wrapping a todo body to the panel width, for todos only.
//!
//! Prose is clipped with an ellipsis because a paragraph that wraps would push
//! the todos the panel exists for off the screen. A todo wraps instead: the whole
//! line is what the user is editing, so hiding half of it is worse than using a
//! second row. Breaks happen between words, so a `#tag` or `@who` is never cut in
//! half and keeps its colour.

use unicode_width::UnicodeWidthStr;

/// The text split into chunks of at most `width` display cells.
///
/// Always at least one chunk, so a caller can rely on `chunks()[0]` and on the
/// chunk count being the number of rows the text needs.
pub fn lines(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![String::new()];
    }
    let mut chunks = Vec::new();
    let mut current = String::new();
    for word in text.split_inclusive(' ') {
        let candidate = format!("{current}{word}");
        if UnicodeWidthStr::width(candidate.as_str()) <= width || current.is_empty() {
            if current.is_empty() && UnicodeWidthStr::width(word) > width {
                // a single word wider than the panel: split it by cells
                let (head, tail) = split(word, width);
                chunks.push(head);
                current.clone_from(&tail);
                continue;
            }
            current = candidate;
        } else {
            chunks.push(current.trim_end().to_owned());
            word.clone_into(&mut current);
        }
    }
    chunks.push(current.trim_end().to_owned());
    chunks
}

/// Text cut at the widest character boundary that fits, and the rest.
fn split(text: &str, width: usize) -> (String, String) {
    let mut head = String::new();
    let mut used = 0;
    for (index, character) in text.char_indices() {
        let character_width = unicode_width::UnicodeWidthChar::width(character).unwrap_or(0);
        if used + character_width > width && !head.is_empty() {
            return (head, text[index..].to_owned());
        }
        head.push(character);
        used += character_width;
    }
    (head, String::new())
}

#[cfg(test)]
mod tests {
    use super::lines;

    #[test]
    fn a_body_that_fits_is_one_chunk() {
        assert_eq!(lines("pay rent #home", 40), ["pay rent #home"]);
        assert_eq!(lines("", 40), [""]);
    }

    #[test]
    fn long_text_breaks_between_words() {
        let text = "one two three four five";
        let chunks = lines(text, 9);
        assert!(chunks.len() > 1, "{chunks:?}");
        for chunk in &chunks {
            assert!(
                unicode_width::UnicodeWidthStr::width(chunk.as_str()) <= 9,
                "{chunk:?}"
            );
        }
        assert_eq!(chunks.join(" "), text);
    }

    #[test]
    fn a_word_wider_than_the_panel_is_split_by_cells() {
        let chunks = lines("wide #tag", 4);
        assert!(chunks.len() > 1, "{chunks:?}");
        for chunk in &chunks {
            assert!(
                unicode_width::UnicodeWidthStr::width(chunk.as_str()) <= 4,
                "{chunk:?}"
            );
        }
    }

    #[test]
    fn a_zero_width_panel_does_not_loop() {
        assert_eq!(lines("anything", 0), [""]);
    }

    #[test]
    fn wide_characters_are_measured_by_display_width() {
        let chunks = lines("日本語のテキスト", 8);
        for chunk in &chunks {
            assert!(
                unicode_width::UnicodeWidthStr::width(chunk.as_str()) <= 8,
                "{chunk:?}"
            );
        }
    }
}
