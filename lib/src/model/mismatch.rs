use std::fmt;

/// The line an edit was anchored to no longer reads as expected, so replaying
/// the edit would write the wrong line. The edit is refused instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mismatch {
    /// Zero-based line index; printed as `line + 1`.
    pub line: usize,
}

impl fmt::Display for Mismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {} changed since this action", self.line + 1)
    }
}

impl std::error::Error for Mismatch {}

#[cfg(test)]
mod tests {
    use super::Mismatch;

    #[test]
    fn names_the_line_that_moved() {
        assert_eq!(
            Mismatch { line: 4 }.to_string(),
            "line 5 changed since this action"
        );
    }
}
