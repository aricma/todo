//! What the loop is built around.

/// How long the loop waits for a key before looking at the file on disk.
///
/// The poll is what makes an external write visible without anyone asking: the
/// file is small, and one `stat` plus one read a second is cheaper than a person
/// wondering why the screen is stale.
pub const TICK: std::time::Duration = std::time::Duration::from_millis(750);
