//! Drawing: the screen as a function of the state, one module per part of it.
//!
//! Nothing here holds state and nothing here decides anything: a renderer reads
//! the app and writes rows. `draw` is the whole screen; the section modules are
//! its pieces.

mod body;
mod collection;
mod consts;
mod draw;
mod files;
mod frame;
mod hints;
mod history;
mod outline;
mod popover;
mod prompt;
mod rows;
mod status;
mod todos;
mod wrap;

pub use draw::draw;
pub use rows::cursor_in;

#[cfg(test)]
mod tests;
