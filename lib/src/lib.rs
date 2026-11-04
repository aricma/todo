//! Read and edit todo lines in markdown files.
//!
//! The rule this crate exists for: a file is a list of exact lines, a task is
//! derived from a single line, and every edit rewrites that one line. Nothing
//! else in the file is ever re-rendered.
#![forbid(unsafe_code)]

pub mod collections;
pub mod discover;
pub mod doc;
pub mod error;
pub mod limits;
pub mod model;
pub mod ops;
pub mod search;
pub mod store;
