//! The remote side's shared vocabulary (ADR-0016), wasm-free so `cargo test`
//! reaches it. Each transport (Drive, WebDAV, S3) has its own URL and signing
//! rules; what they agree on is what a *remote file* is.

/// A remote database file.
///
/// `id` is opaque: how a transport addresses the file (a Drive file id, a URL,
/// an object key). `version` is that transport's concurrency token — Drive's
/// `version` field, or WebDAV/S3's `ETag` — and is `None` when the transport
/// has not read one yet (or cannot).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteFile {
    pub id: String,
    pub version: Option<String>,
}
