use std::fmt;

use crate::model::word::is_word;

/// A `#102` or `#PROJ-404` found in a task body: a ticket number, not a tag.
///
/// A token that is `#` and a word is a **tag** unless the word looks like a ticket:
/// all digits, or capitals and a dash and digits. The shape is what separates the two,
/// because there is nowhere else to put the distinction — `#home` and `#102` sit in the
/// same place in a line (RFC-021).
///
/// The stored text excludes the `#`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ticket(String);

impl Ticket {
    /// Parse a whole token, `#` included. `None` means "not a ticket", and the token is
    /// read as a tag instead.
    pub fn parse(raw: &str) -> Option<Self> {
        let word = raw.strip_prefix('#')?;
        looks_like_a_ticket(word).then(|| Self(word.to_owned()))
    }

    /// The text, without the `#`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Ticket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// Whether a `#` word is a ticket: `102`, `2024`, `PROJ-404`, `JIRA-7`.
///
/// The second shape is strict on purpose — capitals only — because `#worst-bug-of-the-
/// century` is a tag with dashes in it, and lowering the case would make every hyphenated
/// tag whose last word is a number a ticket.
fn looks_like_a_ticket(word: &str) -> bool {
    if !is_word(word) {
        return false;
    }
    if word.chars().all(|character| character.is_ascii_digit()) {
        return true;
    }
    match word.rsplit_once('-') {
        Some((head, tail)) => {
            !head.is_empty()
                && !tail.is_empty()
                && head
                    .chars()
                    .all(|character| character.is_ascii_uppercase() || character == '-')
                && tail.chars().all(|character| character.is_ascii_digit())
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::Ticket;

    #[test]
    fn a_number_is_a_ticket() {
        for raw in ["#102", "#1", "#2024", "#0"] {
            assert_eq!(
                Ticket::parse(raw).map(|t| t.as_str().to_owned()),
                Some(raw[1..].to_owned()),
                "{raw}"
            );
        }
    }

    #[test]
    fn an_upper_case_project_key_and_a_number_is_a_ticket() {
        for raw in ["#PROJ-404", "#JIRA-7", "#AB-12", "#A-1"] {
            assert!(Ticket::parse(raw).is_some(), "{raw}");
        }
    }

    #[test]
    fn a_tag_is_not_a_ticket() {
        for raw in [
            "#home",
            "#worst-bug-of-the-century",
            "#2fa",
            "#proj-404",
            "#infra102",
            "#a-1",
            "#",
            "#two words",
            "102",
            "#PROJ-",
            "#-404",
            // one dash only: a long key like `PROJ-2026-7` is a tag, and a ticket it
            // is not — the shape has to be simple enough to explain in a line
            "#PROJ-1-2",
            "#PROJ-2026-7",
        ] {
            assert_eq!(Ticket::parse(raw), None, "{raw}");
        }
    }
}
