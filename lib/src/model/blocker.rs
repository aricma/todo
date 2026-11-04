/// Why a todo could not move.
///
/// A move is silent only when it is obvious; either of these is not, so the reason
/// is handed back rather than swallowed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blocker {
    /// There is no todo that way: it is already first or last.
    Edge,
    /// A heading is in the way. A todo belongs to the section it is written under,
    /// so moving one past `#` would file it somewhere nobody asked for.
    Heading,
}
