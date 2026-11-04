//! The state machine: three panes, one open file, one session history, and the
//! whole key map.
//!
//! Nothing here draws, and nothing here reads the terminal — the screen is a
//! function of this state, and a key press either changes it or returns an
//! `Action` for the terminal layer to perform. That is what makes every rule
//! testable without a pty.

pub mod action;
pub mod app;
pub mod confirm;
pub mod create;
pub mod entry;
pub mod keys;
pub mod label;
pub mod message;
pub mod prompt;
pub mod retrace;
pub mod row;
pub mod search;

pub use action::{Action, Pane};
pub use app::App;

#[cfg(test)]
mod limit_tests;
#[cfg(test)]
mod tests;
