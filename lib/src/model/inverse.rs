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
    /// Exchange two lines, which must still read as they did. A move is one edit,
    /// so it is one undo, not two.
    Swap {
        a: usize,
        b: usize,
        expect_a: String,
        expect_b: String,
    },
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
            Self::Swap {
                a,
                b,
                expect_a,
                expect_b,
            } => Self::Swap {
                a,
                b,
                expect_a: expect_b,
                expect_b: expect_a,
            },
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
    fn a_swap_flips_its_two_sides() {
        let swap = Inverse::Swap {
            a: 1,
            b: 4,
            expect_a: "a".into(),
            expect_b: "b".into(),
        };
        assert_eq!(
            swap.clone().flip(),
            Inverse::Swap {
                a: 1,
                b: 4,
                expect_a: "b".into(),
                expect_b: "a".into(),
            }
        );
        assert_eq!(swap.clone().flip().flip(), swap);
    }

    #[test]
    fn flipping_twice_is_the_original() {
        assert_eq!(set().flip().flip(), set());
    }
}
