//! Pure date helpers. Calendar math lives in the `time` crate; this module
//! only formats UTC instants. Browser-local date helpers (local_today,
//! local_midnight_utc_iso, now_utc) live in the bin (js_time.rs) — ADR-0004.

/// Format a UTC instant as ISO 8601 (OPS metadata style).
pub fn utc_iso(y: i32, mo: u8, d: u8, h: u8, mi: u8, s: u8) -> String {
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}.000Z")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_iso_formats_zero_padded() {
        assert_eq!(utc_iso(2026, 3, 4, 9, 5, 0), "2026-03-04T09:05:00.000Z");
        assert_eq!(
            utc_iso(2026, 12, 31, 23, 59, 59),
            "2026-12-31T23:59:59.000Z"
        );
    }
}
