//! Drive request vocabulary, wasm-free (ADR-0015, ADR-0009): the query
//! strings, URL builders and response parsing the Drive client needs. The
//! bin does the fetch; everything that can be wrong in a testable way —
//! escaping, `q` construction, reading `files[0].version` — lives here where
//! `cargo test` reaches it.

pub const DRIVE_API: &str = "https://www.googleapis.com/drive/v3";
pub const DRIVE_UPLOAD: &str = "https://www.googleapis.com/upload/drive/v3";
pub const FOLDER_MIME: &str = "application/vnd.google-apps.folder";
pub const DB_MIME: &str = "application/x-sqlite3";

/// A Drive file is a remote file: the shape (id + version token) is shared
/// with the other transports (ADR-0016), so there is one type, not three.
pub use crate::remote::RemoteFile as FileRef;

#[derive(serde::Deserialize)]
struct FileList {
    #[serde(default)]
    files: Vec<RawFile>,
}

#[derive(serde::Deserialize)]
struct RawFile {
    id: String,
    #[serde(default)]
    version: Option<String>,
}

/// Escape a value inside a Drive `q` single-quoted literal.
pub fn q_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "\\'")
}

/// The app's own folder under `parent` (or at the top level when `parent` is
/// `None`). `drive.file` means a name match can only ever be a folder this app
/// created.
pub fn folder_query(name: &str, parent: Option<&str>) -> String {
    let base = format!(
        "name='{}' and mimeType='{}' and trashed=false",
        q_escape(name),
        FOLDER_MIME
    );
    match parent {
        Some(p) => format!("{base} and '{}' in parents", q_escape(p)),
        None => format!("{base} and 'root' in parents"),
    }
}

/// A configured remote root (`application-integration-tests/aricma-todo/…`)
/// split into folder names. `None`/blank uses `default`; empty and `.`
/// segments are dropped; a `..` segment is refused outright (it has no meaning
/// in a Drive folder path), falling back to the default.
pub fn root_segments(raw: Option<&str>, default: &str) -> Vec<String> {
    let raw = raw.map(str::trim).filter(|s| !s.is_empty());
    let Some(raw) = raw else {
        return vec![default.to_string()];
    };
    let mut segments = Vec::new();
    for segment in raw.split('/') {
        let segment = segment.trim();
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." {
            return vec![default.to_string()];
        }
        segments.push(segment.to_string());
    }
    if segments.is_empty() {
        vec![default.to_string()]
    } else {
        segments
    }
}

/// The app's db file inside a known folder.
pub fn db_query(folder_id: &str, db_name: &str) -> String {
    format!(
        "'{}' in parents and name='{}' and trashed=false",
        q_escape(folder_id),
        q_escape(db_name)
    )
}

/// `files.list` URL. `fields` is part of the vocabulary, not the caller's
/// choice: the folder lookup wants ids, the file lookup wants versions too.
pub fn list_url(q: &str, with_version: bool) -> String {
    let fields = if with_version {
        "files(id,version)"
    } else {
        "files(id)"
    };
    format!(
        "{DRIVE_API}/files?q={}&fields={fields}&spaces=drive",
        urlencode(q)
    )
}

pub fn create_folder_url() -> String {
    format!("{DRIVE_API}/files?fields=id")
}

pub fn create_file_url() -> String {
    format!("{DRIVE_API}/files?fields=id,version")
}

pub fn update_media_url(file_id: &str) -> String {
    format!("{DRIVE_UPLOAD}/files/{file_id}?uploadType=media&fields=id,version")
}

/// Metadata for the version guard. `id` is requested alongside `version`
/// because the response is parsed as a `FileRef` — `?fields=version` alone
/// returns `{"version":…}` and parsing it fails on the missing `id`, which
/// made every push after the first one fail.
pub fn metadata_url(file_id: &str) -> String {
    format!("{DRIVE_API}/files/{file_id}?fields=id,version")
}

pub fn download_url(file_id: &str) -> String {
    format!("{DRIVE_API}/files/{file_id}?alt=media")
}

/// Every file in a `files.list` body. An unparseable body is an error, not an
/// empty list that would read as "no folder yet" and create a duplicate.
pub fn parse_files(body: &[u8]) -> Result<Vec<FileRef>, String> {
    let list: FileList = serde_json::from_slice(body).map_err(|e| e.to_string())?;
    Ok(list
        .files
        .into_iter()
        .map(|f| FileRef {
            id: f.id,
            version: f.version,
        })
        .collect())
}

/// The single file a create/metadata response names.
pub fn parse_file(body: &[u8]) -> Result<FileRef, String> {
    let raw: RawFile = serde_json::from_slice(body).map_err(|e| e.to_string())?;
    Ok(FileRef {
        id: raw.id,
        version: raw.version,
    })
}

/// `version` is required by the guard; a response without one is an error,
/// not an empty string that compares equal to nothing.
pub fn required_version(file: &FileRef) -> Result<String, String> {
    file.version
        .clone()
        .ok_or_else(|| "no version in response".to_string())
}

/// `encodeURIComponent` without `js_sys` (the lib is wasm-free). Drive `q`
/// values carry quotes and spaces; `#` and `&` must not survive.
pub fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'!'
            | b'~'
            | b'*'
            | b'\''
            | b'('
            | b')' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_and_backslashes_are_escaped_in_queries() {
        assert_eq!(q_escape("aricma-todo"), "aricma-todo");
        assert_eq!(q_escape("O'Brien"), "O\\'Brien");
        assert_eq!(q_escape("a\\b"), "a\\\\b");
    }

    #[test]
    fn folder_query_names_the_folder_and_the_mime() {
        let q = folder_query("aricma-todo", None);
        assert!(q.contains("name='aricma-todo'"));
        assert!(q.contains(FOLDER_MIME));
        assert!(q.contains("trashed=false"));
        // Top level is scoped to the Drive root, so a nested folder of the
        // same name is not mistaken for it.
        assert!(q.contains("'root' in parents"), "{q}");
    }

    #[test]
    fn folder_query_scopes_to_a_parent_when_given_one() {
        let q = folder_query("drive-live", Some("parent-1"));
        assert!(q.contains("name='drive-live'"));
        assert!(q.contains("'parent-1' in parents"), "{q}");
        assert!(!q.contains("'root' in parents"), "{q}");
    }

    #[test]
    fn root_segments_defaults_and_splits() {
        assert_eq!(root_segments(None, "aricma-todo"), vec!["aricma-todo"]);
        assert_eq!(
            root_segments(Some("  "), "aricma-todo"),
            vec!["aricma-todo"]
        );
        assert_eq!(
            root_segments(Some("app-tests/aricma-todo/suite/test 1"), "x"),
            vec!["app-tests", "aricma-todo", "suite", "test 1"]
        );
        // Empty/dot segments are dropped; `..` is refused (default).
        assert_eq!(root_segments(Some("a//b/./c"), "x"), vec!["a", "b", "c"]);
        assert_eq!(root_segments(Some("a/../b"), "x"), vec!["x"]);
    }

    #[test]
    fn db_query_scopes_to_the_parent_folder() {
        let q = db_query("folder-1", "todos.db");
        assert!(q.contains("'folder-1' in parents"));
        assert!(q.contains("name='todos.db'"));
    }

    #[test]
    fn list_urls_encode_the_query_and_pick_their_fields() {
        let url = list_url(&folder_query("aricma-todo", None), false);
        assert!(url.contains("fields=files(id)&"), "{url}");
        assert!(!url.contains(' '), "spaces must be encoded: {url}");
        let with = list_url(&db_query("f", "todos.db"), true);
        assert!(with.contains("fields=files(id,version)"), "{with}");
    }

    #[test]
    fn the_metadata_url_requests_everything_parse_file_needs() {
        let url = metadata_url("f1");
        assert!(url.contains("fields=id,version"), "{url}");
        // The shape that URL produces must parse as the FileRef callers expect.
        let parsed = parse_file(br#"{"id":"f1","version":"7"}"#).expect("parses");
        assert_eq!(required_version(&parsed).unwrap(), "7");
    }

    #[test]
    fn urlencode_leaves_unreserved_and_escapes_the_rest() {
        assert_eq!(urlencode("name='x'"), "name%3D'x'");
        assert_eq!(urlencode("a b"), "a%20b");
    }

    #[test]
    fn parse_files_reads_ids_and_versions_and_tolerates_absence() {
        let body = br#"{"files":[{"id":"f1","version":"7"},{"id":"f2"}]}"#;
        let files = parse_files(body).expect("parses");
        assert_eq!(
            files[0],
            FileRef {
                id: "f1".into(),
                version: Some("7".into())
            }
        );
        assert_eq!(files[1].version, None);
        assert!(parse_files(br#"{"files":[]}"#).unwrap().is_empty());
    }

    #[test]
    fn a_malformed_body_is_an_error_not_an_empty_match() {
        assert!(parse_files(b"not json").is_err());
        assert!(parse_files(br#"{"files":{}}"#).is_err());
    }

    #[test]
    fn required_version_rejects_a_missing_one() {
        let with = FileRef {
            id: "f".into(),
            version: Some("3".into()),
        };
        assert_eq!(required_version(&with).unwrap(), "3");
        let without = FileRef {
            id: "f".into(),
            version: None,
        };
        assert!(required_version(&without).is_err());
    }
}
