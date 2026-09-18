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

/// The configuration field keys, as one vocabulary. The view renders a form
/// from the storages' `StorageField`s and the transports read the values back;
/// sharing the strings is what keeps those two from drifting apart.
pub mod keys {
    pub const URL: &str = "url";
    pub const USER: &str = "user";
    pub const PASSWORD: &str = "password";
    pub const ENDPOINT: &str = "endpoint";
    pub const BUCKET: &str = "bucket";
    pub const ACCESS_KEY: &str = "access_key";
    pub const SECRET_KEY: &str = "secret_key";
}

/// The database file name inside a remote root. One name for every transport —
/// what changes between them is the root, never the file (ADR-0016).
pub const DB_FILE: &str = "todos.db";

/// A WebDAV remote: the folder the app owns and the credentials for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebDavConfig {
    /// The root URL (a folder). The db lives at `<root>/todos.db`.
    pub url: String,
    pub user: String,
    pub password: String,
}

/// The database's URL inside a WebDAV root. A trailing slash on the root is
/// tolerated — users paste folder URLs both ways.
pub fn db_url(config: &WebDavConfig) -> String {
    format!("{}/{DB_FILE}", config.url.trim_end_matches('/'))
}

/// The `Authorization` header for the WebDAV credentials.
pub fn auth_header(config: &WebDavConfig) -> String {
    format!(
        "Basic {}",
        base64_encode(format!("{}:{}", config.user, config.password).as_bytes())
    )
}

/// Standard base64 with padding (RFC 4648). Hand-rolled because the lib is
/// host-tested and `window.btoa` does not exist off the browser — and it is
/// not worth a dependency for the two dozen bytes Basic auth needs.
pub fn base64_encode(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b1 = chunk[0];
        let b2 = *chunk.get(1).unwrap_or(&0);
        let b3 = *chunk.get(2).unwrap_or(&0);
        let n = (u32::from(b1) << 16) | (u32::from(b2) << 8) | u32::from(b3);
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> WebDavConfig {
        WebDavConfig {
            url: "http://localhost:8080/aricma-todo".to_string(),
            user: "u".to_string(),
            password: "p".to_string(),
        }
    }

    #[test]
    fn webdav_db_url_joins_the_root_and_tolerates_a_trailing_slash() {
        assert_eq!(
            db_url(&config()),
            "http://localhost:8080/aricma-todo/todos.db"
        );
        let with_slash = WebDavConfig {
            url: "http://localhost:8080/aricma-todo/".to_string(),
            ..config()
        };
        assert_eq!(
            db_url(&with_slash),
            "http://localhost:8080/aricma-todo/todos.db"
        );
    }

    #[test]
    fn webdav_auth_is_basic_base64_of_user_colon_password() {
        assert_eq!(auth_header(&config()), "Basic dTpw");
    }

    #[test]
    fn base64_matches_the_rfc_examples() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn base64_covers_every_byte_value() {
        // Pins the alphabet and the padding: a wrong table would still get the
        // RFC examples above right for short ASCII inputs.
        let all: Vec<u8> = (0..=255u8).collect();
        let encoded = base64_encode(&all);
        assert_eq!(encoded.len(), 344);
        assert_eq!(&encoded[..12], "AAECAwQFBgcI");
        // 256 = 85 chunks of 3 + one byte: the tail is padded, not a full group.
        assert!(encoded.ends_with("/w=="), "{encoded}");
    }
}
