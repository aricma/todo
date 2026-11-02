/// One markdown heading, as the outline shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Heading {
    /// Zero-based index into `Doc::lines`.
    pub line: usize,
    /// `#` count: 1 for `#`, 2 for `##`, and so on.
    pub level: u8,
    /// The text after the hashes, without the closing ones.
    pub text: String,
}
