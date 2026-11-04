//! A long line that has to fit one row, keeping its beginning.

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// A line that fits, cut at the *end*: a message says what happened first and
/// trails off, where a path says which file it is last.
pub fn truncate(text: &str, width: usize) -> String {
    if UnicodeWidthStr::width(text) <= width {
        return text.to_owned();
    }
    let budget = width.saturating_sub(1); // one column for the ellipsis
    let mut out = String::new();
    let mut used = 0;
    for character in text.chars() {
        let cell = UnicodeWidthChar::width(character).unwrap_or(0);
        if used + cell > budget {
            break;
        }
        used += cell;
        out.push(character);
    }
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::truncate;

    #[test]
    fn keeps_the_beginning_and_trails_off() {
        assert_eq!(truncate("copied #home", 40), "copied #home");
        assert_eq!(truncate("copied #home", 10), "copied #h…");
    }

    #[test]
    fn a_wide_character_is_two_cells_and_is_not_cut_in_half() {
        // `copied ✅ #` is 11 cells, which leaves no room for the `h`
        assert_eq!(truncate("copied ✅ #home", 12), "copied ✅ #…");
        assert_eq!(truncate("copied ✅ #home", 13), "copied ✅ #h…");
    }
}
