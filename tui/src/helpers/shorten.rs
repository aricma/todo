//! A long string that has to fit one row.

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// A path that fits a status bar, cut from the *front*: the file name is the part
/// that says which file it is, so `…/nested/place/TODOs.md` beats
/// `some/deeply/nested…`.
pub fn shorten(text: &str, width: usize) -> String {
    if UnicodeWidthStr::width(text) <= width {
        return text.to_owned();
    }
    let budget = width.saturating_sub(1); // one column for the leading ellipsis
    let mut kept: Vec<char> = Vec::new();
    let mut used = 0;
    for character in text.chars().rev() {
        let cell = UnicodeWidthChar::width(character).unwrap_or(0);
        if used + cell > budget {
            break;
        }
        used += cell;
        kept.push(character);
    }
    let mut out = String::from("…");
    out.extend(kept.into_iter().rev());
    out
}

#[cfg(test)]
mod tests {
    use super::shorten;

    #[test]
    fn a_long_path_keeps_its_file_name() {
        let short = shorten("some/deeply/nested/place/where/TODOs.md", 20);
        assert!(
            unicode_width::UnicodeWidthStr::width(short.as_str()) <= 20,
            "{short}"
        );
        assert!(short.starts_with('…'), "cut from the front: {short}");
        assert!(
            short.ends_with("TODOs.md"),
            "the file name survives: {short}"
        );
        assert_eq!(shorten("TODOs.md", 20), "TODOs.md");
        assert_eq!(shorten("a/TODOs.md", 20), "a/TODOs.md");
    }

}
