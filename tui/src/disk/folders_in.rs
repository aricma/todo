//! The folders directly inside a path, for a field that is offering them.

use std::path::Path;

/// By name. One that cannot be read offers nothing rather than failing the field
/// it is only suggesting to.
pub fn folders_in(path: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(path) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().to_str().map(ToOwned::to_owned))
        .filter(|name| !name.starts_with('.'))
        .collect();
    names.sort();
    names
}
