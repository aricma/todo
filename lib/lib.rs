//! aricma-todo core: wasm-free logic, fully testable with plain `cargo test`.
//! Everything browser-touching and presentational lives in the bin
//! (app/main.rs) behind the wasm-only deps — see ADR-0009.

pub mod dates;
pub mod model;
pub mod ops;
pub mod parse;
pub mod recurrence;
pub mod states;
