//! aricma-todo core: wasm-free logic, fully testable with plain `cargo test`.
//! Everything browser-touching and presentational lives in the bin
//! (app/main.rs) behind the wasm-only deps — see ADR-0009.

pub mod datepicker;
pub mod dates;
pub mod db_file;
pub mod drive;
/// FTP — a **host-only** transport: a browser cannot open a TCP socket, so this
/// is not compiled into the wasm app. See the module docs in `ftp`.
#[cfg(not(target_arch = "wasm32"))]
pub mod ftp;
pub mod local_place;
pub mod model;
pub mod ops;
pub mod parse;
pub mod recurrence;
pub mod remote;
pub mod states;
