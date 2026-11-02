/// Whether `text` is one or more of `[A-Za-z0-9_-]`.
///
/// The one charset a tag or assignee word may use: no quotes, no spaces, no
/// unicode. Anything else is plain text in the body.
pub(crate) fn is_word(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[cfg(test)]
mod tests {
    use super::is_word;

    #[test]
    fn accepts_letters_digits_dash_and_underscore() {
        for text in ["infra", "has-dash", "a_b2", "12", "A-B_c"] {
            assert!(is_word(text), "{text}");
        }
    }

    #[test]
    fn rejects_empty_and_anything_else() {
        for text in ["", "café", "a b", "a.b", "a\"b", "a/b"] {
            assert!(!is_word(text), "{text}");
        }
    }
}
