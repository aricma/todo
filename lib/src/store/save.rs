//! Writing a file: atomically, and only if nobody else changed it first.

use std::fs;
use std::io;
use std::path::Path;

use super::Loaded;
use crate::error::Error;

/// Replace the file's text, or refuse if it changed since it was read.
///
/// The write goes to a temporary file in the same directory, takes the
/// original's permissions, is flushed to disk, and only then is renamed over
/// the original — so a crash leaves either the old file or the new one.
pub fn save(loaded: &Loaded, text: &str) -> Result<Loaded, Error> {
    if fs::read(&loaded.path)? != loaded.text.as_bytes() {
        return Err(Error::StaleFile(loaded.path.clone()));
    }
    let temporary = temporary_path(&loaded.path);
    let written = write_through(&loaded.path, &temporary, text);
    if written.is_err() {
        let _ = fs::remove_file(&temporary);
        written?;
    }
    Ok(Loaded {
        path: loaded.path.clone(),
        text: text.to_owned(),
    })
}

/// Write the text whatever the file says now.
///
/// The caller has decided: a person was shown that the file changed and chose to
/// keep their version. The write is the same atomic one — only the staleness check
/// is skipped — so choosing "mine" cannot leave a half-written file either.
pub fn overwrite(loaded: &Loaded, text: &str) -> Result<Loaded, Error> {
    let temporary = temporary_path(&loaded.path);
    let written = write_through(&loaded.path, &temporary, text);
    if written.is_err() {
        let _ = fs::remove_file(&temporary);
        written?;
    }
    Ok(Loaded {
        path: loaded.path.clone(),
        text: text.to_owned(),
    })
}

/// Write, match permissions, flush, and rename into place.
fn write_through(original: &Path, temporary: &Path, text: &str) -> io::Result<()> {
    fs::write(temporary, text)?;
    if let Ok(metadata) = fs::metadata(original) {
        fs::set_permissions(temporary, metadata.permissions())?;
    }
    fs::OpenOptions::new()
        .write(true)
        .open(temporary)?
        .sync_all()?;
    fs::rename(temporary, original)
}

fn temporary_path(path: &Path) -> std::path::PathBuf {
    let name = path.file_name().map_or_else(
        || "file".to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );
    path.with_file_name(format!(".{name}.tmp-{}", std::process::id()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn overwrite_ignores_a_file_that_moved() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("TODOs.md");
        std::fs::write(&path, "- [ ] mine\n").unwrap();
        let loaded = super::super::load(&path).unwrap();
        // someone else writes in between
        std::fs::write(&path, "- [ ] theirs\n").unwrap();
        assert!(
            super::save(&loaded, "- [x] mine\n").is_err(),
            "save refuses"
        );
        let after = super::overwrite(&loaded, "- [x] mine\n").expect("a write");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "- [x] mine\n");
        assert_eq!(after.text, "- [x] mine\n");
    }

    use std::fs;

    use super::save;
    use crate::error::Error;
    use crate::store::load;

    fn loaded(root: &std::path::Path) -> crate::store::Loaded {
        let path = root.join("TODOs.md");
        fs::write(&path, "- [ ] a\n").unwrap();
        load(&path).unwrap()
    }

    #[test]
    fn save_replaces_the_file_and_returns_the_new_baseline() {
        let tmp = tempfile::tempdir().unwrap();
        let baseline = loaded(tmp.path());
        let next = save(&baseline, "- [ ] a\n- [x] b\n").unwrap();
        assert_eq!(
            fs::read_to_string(&baseline.path).unwrap(),
            "- [ ] a\n- [x] b\n"
        );
        assert_eq!(next.text, "- [ ] a\n- [x] b\n");
        assert_eq!(next.path, baseline.path);
    }

    #[test]
    fn save_refuses_when_the_file_changed_on_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let baseline = loaded(tmp.path());
        fs::write(&baseline.path, "- [ ] written by someone else\n").unwrap();
        assert!(matches!(
            save(&baseline, "- [ ] a\n- [x] b\n"),
            Err(Error::StaleFile(_))
        ));
        assert_eq!(
            fs::read_to_string(&baseline.path).unwrap(),
            "- [ ] written by someone else\n"
        );
    }

    #[test]
    fn save_leaves_no_temporary_file_behind() {
        let tmp = tempfile::tempdir().unwrap();
        let baseline = loaded(tmp.path());
        save(&baseline, "- [x] a\n").unwrap();
        let names: Vec<String> = fs::read_dir(tmp.path())
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["TODOs.md"]);
    }

    #[cfg(unix)]
    #[test]
    fn save_preserves_the_file_mode() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let baseline = loaded(tmp.path());
        fs::set_permissions(&baseline.path, fs::Permissions::from_mode(0o640)).unwrap();
        save(&baseline, "- [x] a\n").unwrap();
        let mode = fs::metadata(&baseline.path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o640);
    }
}
