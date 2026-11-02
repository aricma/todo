pub mod load;
pub mod save;

pub use load::load;
pub use save::save;

/// A file as it was read, and the text the next save must still start from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Loaded {
    pub path: std::path::PathBuf,
    pub text: String,
}
