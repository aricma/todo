//! What a **db file** has to look like before this app will hand it to SQLite.
//!
//! The pull path replaces the local store with whatever bytes are at the remote's
//! database path, and a remote is not necessarily serving a whole file: a PUT in
//! flight, an interrupted upload, or a half-written file left by something else
//! all produce a file whose first 16 bytes are a perfect SQLite header and whose
//! body is short. Handed to SQLite that is `SQLITE_NOTADB` — "file is not a
//! database" (measured, desktop and webkit) — or, worse, a store replaced by a
//! partial database, which only fails when a query reads past what arrived.
//!
//! So the header is *parsed* here, not pattern-matched: the magic says a file
//! tries to be a database, and the page size and page count say whether all of it
//! arrived. This is plain data — no db, no browser — so it is host-tested.

/// The 16 bytes every SQLite database starts with.
const MAGIC: &[u8; 16] = b"SQLite format 3\0";
/// A SQLite database header is 100 bytes; a shorter file cannot be one even when
/// it starts with the magic (a truncated download used to pass that check).
const HEADER_LEN: usize = 100;
/// Header offsets (SQLite file format, §1.2).
const PAGE_SIZE_AT: usize = 16;
const PAGE_COUNT_AT: usize = 28;
/// The largest page size SQLite allows, spelled as `1` in the header.
const MAX_PAGE_SIZE: u32 = 65_536;
/// "The page count is not known yet" (a db mid-WAL), and the honest way to say
/// "no value" — both mean the header cannot tell us the file's size.
const PAGE_COUNT_UNKNOWN: u32 = 0xFFFF_FFFF;

/// Whether these bytes are a **complete** SQLite database — a usable header and a
/// body as long as the header says. `Err` is the reason, written for the user:
/// the caller shows it instead of letting SQLite report a file it cannot read.
pub fn check_db_bytes(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() < HEADER_LEN || &bytes[..MAGIC.len()] != MAGIC {
        return Err(format!(
            "the remote file is not a SQLite database ({} bytes) — nothing was imported",
            bytes.len()
        ));
    }

    let page_size = match u16::from_be_bytes([bytes[PAGE_SIZE_AT], bytes[PAGE_SIZE_AT + 1]]) {
        // SQLite spells 65536 as 1: the field is 16 bits and 65536 does not fit.
        1 => MAX_PAGE_SIZE,
        other => u32::from(other),
    };
    if !(512..=MAX_PAGE_SIZE).contains(&page_size) || !page_size.is_power_of_two() {
        return Err(format!(
            "the remote file's header is unreadable (page size {page_size}) — nothing was imported"
        ));
    }

    if !bytes.len().is_multiple_of(page_size as usize) {
        return Err(format!(
            "the remote file is incomplete: {} bytes is not a whole number of {page_size}-byte pages \
             — nothing was imported",
            bytes.len()
        ));
    }

    let pages = u32::from_be_bytes([
        bytes[PAGE_COUNT_AT],
        bytes[PAGE_COUNT_AT + 1],
        bytes[PAGE_COUNT_AT + 2],
        bytes[PAGE_COUNT_AT + 3],
    ]);
    if pages != 0 && pages != PAGE_COUNT_UNKNOWN {
        let expected = u64::from(pages) * u64::from(page_size);
        if (bytes.len() as u64) < expected {
            return Err(format!(
                "the remote file is incomplete: its header declares {pages} pages ({expected} bytes) \
                 but {} bytes arrived — nothing was imported",
                bytes.len()
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A database header, with `pages` pages declared and `len` bytes of body.
    fn file(len: usize, page_size: u16, pages: u32) -> Vec<u8> {
        let mut v = vec![0u8; len.max(HEADER_LEN)];
        v[..MAGIC.len()].copy_from_slice(MAGIC);
        v[PAGE_SIZE_AT..PAGE_SIZE_AT + 2].copy_from_slice(&page_size.to_be_bytes());
        v[PAGE_COUNT_AT..PAGE_COUNT_AT + 4].copy_from_slice(&pages.to_be_bytes());
        v
    }

    #[test]
    fn a_whole_db_passes() {
        assert!(check_db_bytes(&file(4096, 4096, 1)).is_ok());
        assert!(check_db_bytes(&file(8192, 4096, 2)).is_ok());
        // 65536 is spelled `1` in the header.
        assert!(check_db_bytes(&file(65536, 1, 1)).is_ok());
        // A page count the header cannot state yet: the length check still holds.
        assert!(check_db_bytes(&file(8192, 4096, PAGE_COUNT_UNKNOWN)).is_ok());
        assert!(check_db_bytes(&file(8192, 4096, 0)).is_ok());
    }

    #[test]
    fn a_file_that_is_not_a_db_is_refused() {
        // The measured case: a marker file a test wrote, 15 bytes of the magic.
        let marker = b"SQLite format 3".to_vec();
        let err = check_db_bytes(&marker).unwrap_err();
        assert!(err.contains("not a SQLite database"), "{err}");
        assert!(err.contains("15 bytes"), "{err}");

        // Any other content at that path (an HTML error page, a directory
        // listing) is refused the same way.
        let html = b"<!doctype html><html><body>404</body></html>".to_vec();
        assert!(check_db_bytes(&html).is_err());
    }

    #[test]
    fn a_partial_download_is_refused_before_sqlite_sees_it() {
        // A body shorter than the header's own page count: the file's first
        // bytes are a perfect header, which is what let this reach SQLite.
        let err = check_db_bytes(&file(4096, 4096, 14)).unwrap_err();
        assert!(err.contains("incomplete"), "{err}");
        assert!(err.contains("14 pages"), "{err}");
        assert!(err.contains("57344 bytes"), "{err}");

        // A cut in the middle of a page: not a whole number of pages at all.
        let err = check_db_bytes(&file(5000, 4096, 14)).unwrap_err();
        assert!(err.contains("incomplete"), "{err}");
        assert!(err.contains("4096-byte pages"), "{err}");
    }

    #[test]
    fn an_unreadable_header_is_refused() {
        // Page size 1000: not a power of two, so not a SQLite page size.
        let err = check_db_bytes(&file(4096, 1000, 1)).unwrap_err();
        assert!(err.contains("unreadable"), "{err}");
        // Page size 256: a power of two, below SQLite's floor.
        assert!(check_db_bytes(&file(4096, 256, 1)).is_err());
    }
}
