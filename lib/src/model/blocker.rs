/// Why a todo could not move.
///
/// A move is silent only when it is obvious; this is not, so the reason is handed
/// back rather than swallowed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blocker {
    /// There is no todo that way: it is already first or last.
    Edge,
}
