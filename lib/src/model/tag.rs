use std::fmt;

use crate::model::word::is_word;

/// A `#tag` found in a task body. The stored text excludes the `#`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tag(String);

impl Tag {
    /// Parse a whole token, `#` included. `None` means "not a tag", and the
    /// token stays plain text in the body.
    pub fn parse(raw: &str) -> Option<Self> {
        let word = raw.strip_prefix('#')?;
        is_word(word).then(|| Self(word.to_owned()))
    }

    /// The word, without the `#`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Tag;

    #[test]
    fn parses_a_word_tag() {
        assert_eq!(Tag::parse("#infra").unwrap().as_str(), "infra");
    }

    #[test]
    fn allows_dashes_digits_and_underscores() {
        for raw in ["#has-dash", "#a_b2", "#12"] {
            assert!(Tag::parse(raw).is_some(), "{raw}");
        }
    }

    #[test]
    fn rejects_everything_else() {
        for raw in ["#café", "#two words", "#", "infra", "#a\"b", "#a/b"] {
            assert_eq!(Tag::parse(raw), None, "{raw}");
        }
    }

    #[test]
    fn displays_with_the_leading_hash() {
        assert_eq!(Tag::parse("#infra").unwrap().to_string(), "#infra");
    }
}
