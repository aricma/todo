//! Matches a query against a line, the way a person expects a filter box to.
//!
//! Case-insensitive, and the query's characters must appear in order but not
//! adjacently — `pyrnt` finds `pay rent`. Contiguous runs score higher than
//! scattered ones, a match at a word start scores higher still, and an earlier
//! match beats a later one. That is enough to put `pay rent` above
//! `prepare rent` for `pr`, without a matcher crate: the whole thing is one
//! greedy pass, and it is tested.

/// How much a match is worth, or `None` when the query does not match at all.
///
/// An empty query matches everything with score 0, so callers can show the
/// whole list before anything is typed.
pub fn score(query: &str, text: &str) -> Option<u32> {
    if query.is_empty() {
        return Some(0);
    }
    let needles: Vec<char> = query.chars().flat_map(char::to_lowercase).collect();
    let haystack: Vec<char> = text.chars().flat_map(char::to_lowercase).collect();
    let mut score: u32 = 0;
    let mut matched = 0;
    let mut previous: Option<usize> = None;
    for (index, character) in haystack.iter().enumerate() {
        if matched == needles.len() {
            break;
        }
        if *character != needles[matched] {
            continue;
        }
        score += match previous {
            // adjacent to the previous match: a run in progress
            Some(previous) if previous + 1 == index => 3,
            _ => 1,
        };
        if index == 0 || !haystack[index - 1].is_alphanumeric() {
            // the start of a word, which is what a person is aiming at
            score += 2;
        }
        previous = Some(index);
        matched += 1;
    }
    if matched == needles.len() {
        // Where the match starts matters, but it must not be able to cancel the
        // bonuses: multiply the bonuses out of reach of a small penalty.
        let position = first_index(&haystack, needles[0]).min(9);
        Some(score.saturating_mul(10).saturating_sub(position))
    } else {
        None
    }
}

fn first_index(haystack: &[char], character: char) -> u32 {
    haystack
        .iter()
        .position(|candidate| *candidate == character)
        .and_then(|index| u32::try_from(index).ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::score;

    #[test]
    fn an_empty_query_matches_everything() {
        assert_eq!(score("", "pay rent"), Some(0));
    }

    #[test]
    fn the_characters_must_appear_in_order() {
        assert!(score("pyrnt", "pay rent").is_some());
        assert_eq!(score("rentpay", "pay rent"), None);
        assert_eq!(score("xyz", "pay rent"), None);
    }

    #[test]
    fn case_does_not_matter() {
        assert!(score("PAY", "pay rent").is_some());
        assert!(score("pay", "PAY RENT").is_some());
    }

    #[test]
    fn a_contiguous_match_outscores_a_scattered_one() {
        let contiguous = score("pay", "pay rent").unwrap();
        let scattered = score("pay", "prepare apples yearly").unwrap();
        assert!(contiguous > scattered, "{contiguous} vs {scattered}");
    }

    #[test]
    fn a_word_start_outscores_the_middle_of_a_word() {
        let word_start = score("r", "pay rent").unwrap();
        let middle = score("r", "prepared").unwrap();
        assert!(word_start > middle, "{word_start} vs {middle}");
    }

    #[test]
    fn an_earlier_match_outscores_a_later_one() {
        let early = score("a", "apple pie").unwrap();
        let late = score("a", "the last apple").unwrap();
        assert!(early > late, "{early} vs {late}");
    }

    #[test]
    fn multibyte_text_is_matched_by_character() {
        assert!(score("café", "le café").is_some());
        assert!(score("cafe", "le café").is_none(), "no accent folding");
    }
}
