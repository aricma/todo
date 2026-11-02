/// One rule violation, as `--check` reports it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    /// Zero-based line index; printed as `line + 1`.
    pub line: usize,
    pub message: String,
}
