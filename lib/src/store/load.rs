//! Reading a file into the text the editor works on.

use std::fs;
use std::path::Path;

use super::Loaded;
use crate::error::Error;

/// Read a file as UTF-8. Never lossy: bytes that are not UTF-8 are an error.
pub fn load(path: &Path) -> Result<Loaded, Error> {
    let bytes = fs::read(path)?;
    let text = String::from_utf8(bytes).map_err(|_| Error::NotUtf8(path.to_path_buf()))?;
    Ok(Loaded {
        path: path.to_path_buf(),
        text,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::load;
    use crate::error::Error;

    #[test]
    fn reads_a_file_and_keeps_its_text_verbatim() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("TODOs.md");
        fs::write(&path, "# t\n\n- [ ] a\n").unwrap();
        let loaded = load(&path).unwrap();
        assert_eq!(loaded.path, path);
        assert_eq!(loaded.text, "# t\n\n- [ ] a\n");
    }

    #[test]
    fn a_non_utf8_file_is_an_error_not_a_lossy_read() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("TODOs.md");
        fs::write(&path, [0xff, 0xfe]).unwrap();
        assert!(matches!(load(&path), Err(Error::NotUtf8(_))));
    }

    #[test]
    fn a_missing_file_is_an_io_error() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(matches!(
            load(&tmp.path().join("nope.md")),
            Err(Error::Io(_))
        ));
    }
}
