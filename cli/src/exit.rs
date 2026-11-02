//! Exit codes, which are the CLI's contract with a script.

/// Everything asked for was done.
pub const OK: u8 = 0;
/// `--check` found a rule violation.
pub const FINDINGS: u8 = 1;
/// The command line or the filesystem said no.
pub const USAGE: u8 = 2;
