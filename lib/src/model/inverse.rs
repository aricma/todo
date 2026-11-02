/// The edit a mutation performed, kept so it can be replayed backwards.
///
/// `Inverse::flip` is the whole undo mechanism: the history stores the edit that
/// was applied and replays its opposite, which is why an entry can be
/// unchecked and checked again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Inverse {
    /// Replace one line, which must still read `expect`.
    Set {
        line: usize,
        expect: String,
        text: String,
    },
    /// Insert one line at `line`.
    Insert { line: usize, text: String },
    /// Remove the line at `line`, which must still read `expect`.
    Remove { line: usize, expect: String },
}

impl Inverse {
    /// The edit that undoes this one.
    #[must_use]
    pub fn flip(self) -> Self {
        match self {
            Self::Set { line, expect, text } => Self::Set {
                line,
                expect: text,
                text: expect,
            },
            Self::Insert { line, text } => Self::Remove { line, expect: text },
            Self::Remove { line, expect } => Self::Insert { line, text: expect },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Inverse;

    fn set() -> Inverse {
        Inverse::Set {
            line: 3,
            expect: "a".into(),
            text: "b".into(),
        }
    }

    #[test]
    fn flip_swaps_directions() {
        assert_eq!(
            set().flip(),
            Inverse::Set {
                line: 3,
                expect: "b".into(),
                text: "a".into(),
            }
        );
        assert_eq!(
            Inverse::Insert {
                line: 1,
                text: "x".into(),
            }
            .flip(),
            Inverse::Remove {
                line: 1,
                expect: "x".into(),
            }
        );
        assert_eq!(
            Inverse::Remove {
                line: 1,
                expect: "x".into(),
            }
            .flip(),
            Inverse::Insert {
                line: 1,
                text: "x".into(),
            }
        );
    }

    #[test]
    fn flipping_twice_is_the_original() {
        assert_eq!(set().flip().flip(), set());
    }
}
