//! A path as the screen shows it. Windows hands out `\`, and the rows, the header
//! and the recorded screens have to read the same wherever the tool runs.

use std::path::Path;

/// The path with `/` separators, so `a\TODOs.md` reads as `a/TODOs.md`.
pub fn shown_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}
