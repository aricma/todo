//! Small pure functions used by more than one layer.
//!
//! One function per file: a helper is worth its own name, and its name is worth
//! being findable. Nothing here knows about the state machine, the screen or the
//! disk.

mod as_u16;
mod base64;
mod shift;
mod shorten;
mod shown_path;
mod split_path;
mod truncate;

pub use as_u16::as_u16;
pub use base64::base64;
pub use shift::shift;
pub use shorten::shorten;
pub use shown_path::shown_path;
pub use split_path::split_path;
pub use truncate::truncate;
