//! How much of a tree this tool will look at.
//!
//! A todo tool is pointed at a repository, and a repository is unbounded: a
//! nested `vendor/`, a generated folder nobody excluded, a single 200k-line
//! changelog. These are the numbers the scan stops at, so a mistaken path (or a
//! very large checkout) fails predictably instead of eating the terminal's
//! memory and the user's afternoon. Exceeding one is reported, never silent.
//!
//! Each is a constant in one place because the CLI and the TUI must agree on
//! what "the files" means — `--check` walking further than the screen would be
//! the worst kind of disagreement.

/// Folders below the path given on the command line. The path itself is depth 0,
/// so a file in its tenth nested folder is still found and an eleventh is not.
pub const MAX_DEPTH: usize = 10;

/// Candidate files kept from one scan. Reaching it stops the walk and warns.
pub const MAX_FILES: usize = 1_000;

/// Lines in one file. A longer file is listed, but marked and never opened: the
/// tool will not silently show half a document.
pub const MAX_LINES: usize = 10_000;

/// Bytes in a file the walk will open. Every file in the tree is read once, because
/// a `TODO:` comment can be in any of them, and a lockfile or a minified bundle can
/// be enormous; anything bigger is skipped unread.
pub const MAX_BYTES: u64 = 1 << 20;

/// Files opened in one walk. Lower than a big tree's file count on purpose: a
/// vendored folder that git does not ignore must not cost the reader its afternoon,
/// and reaching this is reported rather than hidden.
pub const MAX_READ: usize = 20_000;
