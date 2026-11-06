/// The word that opens a todo comment, and how loudly it says it.
///
/// Four words, from the oldest comment convention there is: `TODO` is a thing to do,
/// `FIXME` is a thing that is wrong, `HACK` is a thing done knowingly, `XXX` is a thing
/// to look at again. They are all todos to this tool — a line it tracks and can edit —
/// and the word is kept so the line still says which kind it is (RFC-020).
///
/// The words are matched **in capitals only**. A lowercase `fixme:` is a sentence that
/// talks about one, and the rule that reads these lines guesses narrow on purpose
/// (RFC-019).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Todo,
    Fixme,
    Hack,
    Xxx,
}

impl Severity {
    /// Every word, in the order a tie at the same offset is broken.
    pub const ALL: [Self; 4] = [Self::Todo, Self::Fixme, Self::Hack, Self::Xxx];

    /// The word as it is written in a line, in capitals.
    pub fn word(self) -> &'static str {
        match self {
            Self::Todo => "TODO",
            Self::Fixme => "FIXME",
            Self::Hack => "HACK",
            Self::Xxx => "XXX",
        }
    }

    /// Whether `text` opens with this severity and its colon.
    pub fn opens(self, text: &str) -> bool {
        text.strip_prefix(self.word())
            .is_some_and(|rest| rest.starts_with(':'))
    }
}

#[cfg(test)]
mod tests {
    use super::Severity;

    #[test]
    fn every_word_opens_its_own_colon_and_nothing_else() {
        for severity in Severity::ALL {
            assert!(
                severity.opens(&format!("{}: x", severity.word())),
                "{severity:?}"
            );
            assert!(
                severity.opens(&format!("{}:x", severity.word())),
                "{severity:?}"
            );
            assert!(!severity.opens(severity.word()), "{severity:?}");
            assert!(
                !severity.opens(&format!("{} x", severity.word())),
                "{severity:?}"
            );
        }
    }

    #[test]
    fn a_lowercase_word_is_a_sentence_and_not_a_severity() {
        for severity in Severity::ALL {
            let lower = severity.word().to_lowercase();
            assert!(!severity.opens(&format!("{lower}: x")), "{severity:?}");
        }
    }

    #[test]
    fn no_word_is_a_prefix_of_another() {
        for left in Severity::ALL {
            for right in Severity::ALL {
                if left != right {
                    assert!(!right.word().starts_with(left.word()), "{left:?} {right:?}");
                }
            }
        }
    }
}
