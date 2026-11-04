//! The terminal layer: the screen, the event loop, and the one place an `Action`
//! becomes something real.
//!
//! What a key means belongs to `state`, what gets drawn belongs to `ui`. Here the
//! order is always the same: draw the state, wait for an event, hand it to the
//! state machine, perform what comes back.
//!
//! It also owns the terminal's own settings — the alternate screen, the keyboard
//! protocol — so that everything it turns on is turned off again, on the way out
//! and from the panic hook.

pub mod args;

mod consts;
mod enhanced;
mod fail;
mod hook;
mod perform;
mod pump;
mod run;
mod setup;

pub use fail::fail;
pub use run::run;
pub use setup::setup;

#[cfg(test)]
mod tests;
