//! `web/src/TODOs.md` is a folder and a name, and the row that draws it wants
//! both.

/// Split at the last separator: `("web/src/", "TODOs.md")`. A path with no
/// separator has no folder part and the whole thing is the name.
pub fn split_path(path: &str) -> (String, String) {
    path.rfind('/').map_or_else(
        || (String::new(), path.to_owned()),
        |at| (path[..=at].to_owned(), path[at + 1..].to_owned()),
    )
}
