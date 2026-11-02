use std::fmt;

use crate::model::word::is_word;

/// An `@assignee` found in a task body. The stored text excludes the `@`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assignee(String);

impl Assignee {
    /// Parse a whole token, `@` included. `None` means "not an assignee", and
    /// the token stays plain text in the body.
    pub fn parse(raw: &str) -> Option<Self> {
        let word = raw.strip_prefix('@')?;
        is_word(word).then(|| Self(word.to_owned()))
    }

    /// The word, without the `@`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Assignee {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Assignee;

    #[test]
    fn parses_a_word_assignee() {
        assert_eq!(Assignee::parse("@aricma").unwrap().as_str(), "aricma");
    }

    #[test]
    fn allows_dashes_digits_and_underscores() {
        for raw in ["@has-dash", "@a_b2", "@12"] {
            assert!(Assignee::parse(raw).is_some(), "{raw}");
        }
    }

    #[test]
    fn rejects_everything_else() {
        for raw in [
            "@café",
            "@two words",
            "@",
            "aricma",
            "@a\"b",
            "bob@host.com",
        ] {
            assert_eq!(Assignee::parse(raw), None, "{raw}");
        }
    }

    #[test]
    fn displays_with_the_leading_at() {
        assert_eq!(Assignee::parse("@aricma").unwrap().to_string(), "@aricma");
    }
}
