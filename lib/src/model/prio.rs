/// Priority of a task: the letter that may open its body.
///
/// `A` is important and urgent, `D` neither — the four classes of the
/// Eisenhower matrix, in the order the OPS standard recommends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Prio {
    A,
    B,
    C,
    D,
}

impl Prio {
    /// The priority a letter names, in either case.
    pub fn from_letter(letter: char) -> Option<Self> {
        match letter.to_ascii_uppercase() {
            'A' => Some(Self::A),
            'B' => Some(Self::B),
            'C' => Some(Self::C),
            'D' => Some(Self::D),
            _ => None,
        }
    }

    /// The canonical, upper case letter.
    pub fn letter(self) -> char {
        match self {
            Self::A => 'A',
            Self::B => 'B',
            Self::C => 'C',
            Self::D => 'D',
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Prio;

    #[test]
    fn letters_round_trip() {
        for (letter, prio) in [
            ('A', Prio::A),
            ('b', Prio::B),
            ('C', Prio::C),
            ('d', Prio::D),
        ] {
            assert_eq!(Prio::from_letter(letter), Some(prio));
            assert_eq!(prio.letter(), letter.to_ascii_uppercase());
        }
    }

    #[test]
    fn other_characters_are_not_priorities() {
        assert_eq!(Prio::from_letter('E'), None);
        assert_eq!(Prio::from_letter('1'), None);
        assert_eq!(Prio::from_letter(' '), None);
    }
}
