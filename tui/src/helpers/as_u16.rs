//! A `usize` a terminal can be told about: widths and offsets are counted in
//! columns, and a buffer is as wide as a `u16` says.

/// As much of `value` as fits a coordinate; anything wider is off screen anyway.
pub fn as_u16(value: usize) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}
