//! The file operations the tool performs on its own.
//!
//! Reading and writing a todo *file* is the lib's job — `store` parses it, refuses
//! to lose a stale edit, and knows what a valid document is; `discover` walks a
//! tree. This module owns only what the lib has no opinion about: making an empty
//! file without ever overwriting one, removing one, and listing the folders inside
//! one. Wrapping `store::load` here would be a second name for one
//! call, so it is not wrapped.
//!
//! The state machine calls these and stores what comes back: a read is a value it
//! puts in `App`, a write is an `Action` the terminal layer performs.

mod create;
mod delete;
mod folders_in;

pub use create::create;
pub use delete::delete;
pub use folders_in::folders_in;
