//! Moving a cursor inside a list, and stopping at its ends.

/// Move by `delta` inside `0..len`, clamped.
///
/// A negative `delta` counts back from the current row, so a list that shrank
/// under the cursor keeps it on the last row that still exists.
pub fn shift(current: usize, delta: isize, len: usize) -> usize {
    let last = len.saturating_sub(1);
    if delta < 0 {
        current.saturating_sub(delta.unsigned_abs())
    } else {
        (current + delta.unsigned_abs()).min(last)
    }
}
