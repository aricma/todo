use std::fmt;
use std::io;
use std::path::PathBuf;

/// Everything that can go wrong in this crate.
#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    /// A file that is not UTF-8; it is never decoded lossily.
    NotUtf8(PathBuf),
    /// The file changed on disk since it was loaded.
    StaleFile(PathBuf),
    /// An OPS document this tool cannot read or write, and why.
    Ops(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::NotUtf8(path) => write!(f, "{}: not UTF-8", path.display()),
            Self::StaleFile(path) => {
                write!(f, "{}: changed on disk since it was loaded", path.display())
            }
            Self::Ops(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::NotUtf8(_) | Self::StaleFile(_) | Self::Ops(_) => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
