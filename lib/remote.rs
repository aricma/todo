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

/// The configuration field keys, as one vocabulary. A host renders a form
/// from the storages' field lists (`model::form`) and the transports read them
/// sharing the strings is what keeps those two from drifting apart.
pub mod keys {
    /// Optional: what the user calls this source. Purely a name — the store is
    /// named after the remote's root, so renaming never makes a second store.
    pub const NAME: &str = "name";
    pub const URL: &str = "url";
    /// Optional: a path prefix under the root, *before* the app's namespace
    /// folder (ADR-0019). Empty keeps the namespace directly under the root.
    pub const PATH: &str = "path";
    pub const USER: &str = "user";
    pub const PASSWORD: &str = "password";
    pub const ENDPOINT: &str = "endpoint";
    pub const BUCKET: &str = "bucket";
    pub const ACCESS_KEY: &str = "access_key";
    pub const SECRET_KEY: &str = "secret_key";
}

/// The database file name inside a remote namespace. One name for every
/// transport — what changes between them is the root, never the file
/// (ADR-0016).
pub const DB_FILE: &str = "todos.db";

/// The app's namespace folder on a remote: everything the app keeps there —
/// today `todos.db`, later attachments (ADR-0011) — lives under this one name,
/// and the app creates it where the protocol allows (ADR-0019). It is the same
/// name as the local OPFS source directory and Drive's folder (see
/// `app::constants::SOURCE_DIR`, which points here).
pub const SOURCE_DIR: &str = "aricma-todo";

/// A WebDAV remote: the root, an optional path prefix, and the credentials.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebDavConfig {
    /// The root URL the user configured (a folder).
    pub url: String,
    /// An optional path prefix under the root, before the app's namespace folder
    /// (ADR-0019): `/some/path` gives `<root>/some/path/aricma-todo/todos.db`.
    pub path: String,
    pub user: String,
    pub password: String,
}

/// The URL of the app's namespace folder inside a WebDAV root: the configured
/// root, the optional path prefix, then [`SOURCE_DIR`]. Slashes and space
/// around either half are tolerated — users paste folder URLs both ways.
pub fn dir_url(config: &WebDavConfig) -> String {
    let root = config.url.trim().trim_end_matches('/');
    let path = config.path.trim().trim_matches('/');
    if path.is_empty() {
        format!("{root}/{SOURCE_DIR}")
    } else {
        format!("{root}/{path}/{SOURCE_DIR}")
    }
}

/// The database's URL inside a WebDAV namespace: `<dir_url>/todos.db`.
pub fn db_url(config: &WebDavConfig) -> String {
    format!("{}/{DB_FILE}", dir_url(config))
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

/// An S3 remote: a path-style endpoint, one bucket, an optional key prefix, and
/// the keys for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct S3Config {
    /// The endpoint, no trailing slash and no bucket (`https://s3.example.com`).
    pub endpoint: String,
    pub bucket: String,
    /// An optional key prefix under the bucket, before the app's namespace folder
    /// (ADR-0019). Empty keeps the namespace directly in the bucket.
    pub path: String,
    pub access_key: String,
    pub secret_key: String,
}

/// The region the signature claims. MinIO and SeaweedFS accept `us-east-1` for
/// any bucket, and a self-hosted remote is the whole point of this transport;
/// a real AWS bucket elsewhere would need this to become a configuration field.
pub const S3_REGION: &str = "us-east-1";

/// The SigV4 service name.
pub const S3_SERVICE: &str = "s3";

/// The SigV4 algorithm identifier, as it appears in the `Authorization` header.
pub const S3_ALGORITHM: &str = "AWS4-HMAC-SHA256";

/// SHA-256 of the empty string — the payload hash for GET and HEAD.
pub const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

/// The app's object URL: path-style,
/// `{endpoint}/{bucket}/<path>/aricma-todo/todos.db`.
pub fn object_url(config: &S3Config) -> String {
    format!(
        "{}{}",
        config.endpoint.trim_end_matches('/'),
        object_path(config)
    )
}

/// The canonical URI for that object — what the signature covers. `path` is the
/// user's prefix; the app's namespace folder and the db file always follow it
/// (ADR-0019).
pub fn object_path(config: &S3Config) -> String {
    let prefix = config.path.trim().trim_matches('/');
    if prefix.is_empty() {
        format!("/{}/{SOURCE_DIR}/{DB_FILE}", config.bucket)
    } else {
        format!("/{}/{}/{SOURCE_DIR}/{DB_FILE}", config.bucket, prefix)
    }
}

/// The authority of an endpoint URL, which S3 signs as the `Host` header.
pub fn host_of(endpoint: &str) -> String {
    endpoint
        .split("://")
        .nth(1)
        .unwrap_or(endpoint)
        .split('/')
        .next()
        .unwrap_or_default()
        .to_string()
}

/// The SigV4 canonical request. Byte-exact matters: every line ends with a
/// newline, the header block's last newline is *not* doubled, and the query
/// string is empty for our object requests.
pub fn canonical_request(
    method: &str,
    canonical_uri: &str,
    host: &str,
    payload_sha256: &str,
    amz_date: &str,
) -> String {
    format!(
        "{method}\n{canonical_uri}\n\n\
         host:{host}\n\
         x-amz-content-sha256:{payload_sha256}\n\
         x-amz-date:{amz_date}\n\n\
         {SIGNED_HEADERS}\n{payload_sha256}"
    )
}

/// The headers SigV4 covers, sorted, as they appear in `SignedHeaders`.
pub const SIGNED_HEADERS: &str = "host;x-amz-content-sha256;x-amz-date";

/// The SigV4 string to sign: algorithm, timestamp, scope, then the hash of the
/// canonical request.
pub fn string_to_sign(canonical_request: &str, amz_date: &str) -> String {
    let date = amz_date.get(..8).unwrap_or_default();
    format!(
        "{S3_ALGORITHM}\n{amz_date}\n{date}/{}/{S3_SERVICE}/aws4_request\n{}",
        S3_REGION,
        sha256_hex(canonical_request.as_bytes())
    )
}

/// The headers a signed request must send, `Authorization` included.
///
/// SigV4: hash the canonical request, hash that into a string to sign with the
/// date and scope, then sign it with a key derived from the secret by four
/// HMACs. `canonical_uri` is [`object_path`]; `payload_sha256` is
/// [`EMPTY_SHA256`] for a body-less request; `amz_date` is `YYYYMMDDTHHMMSSZ`.
pub fn sign_request(
    config: &S3Config,
    method: &str,
    canonical_uri: &str,
    payload_sha256: &str,
    amz_date: &str,
) -> Vec<(String, String)> {
    let host = host_of(&config.endpoint);
    let date = amz_date.get(..8).unwrap_or_default();
    let canonical = canonical_request(method, canonical_uri, &host, payload_sha256, amz_date);
    let to_sign = string_to_sign(&canonical, amz_date);
    let signature = hex(&hmac(&signing_key(&config.secret_key, date), &to_sign));
    let authorization = format!(
        "{S3_ALGORITHM} Credential={}/{date}/{}/{S3_SERVICE}/aws4_request, \
         SignedHeaders={SIGNED_HEADERS}, Signature={signature}",
        config.access_key, S3_REGION
    )
    .replace("         ", "");

    vec![
        ("Host".to_string(), host),
        (
            "x-amz-content-sha256".to_string(),
            payload_sha256.to_string(),
        ),
        ("x-amz-date".to_string(), amz_date.to_string()),
        ("Authorization".to_string(), authorization),
    ]
}

/// The SigV4 signing key: the secret through four HMACs (date, region,
/// service, terminator). Deriving it once per request is what the algorithm
/// specifies; it is cheap.
fn signing_key(secret: &str, date: &str) -> Vec<u8> {
    let k_date = hmac(format!("AWS4{secret}").as_bytes(), date);
    let k_region = hmac(&k_date, S3_REGION);
    let k_service = hmac(&k_region, S3_SERVICE);
    hmac(&k_service, "aws4_request")
}

fn hmac(key: &[u8], data: &str) -> Vec<u8> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC takes any key length");
    mac.update(data.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

/// Hex SHA-256 of `bytes` — the payload hash SigV4 signs.
/// The name a source's local store goes by, given what identifies that source.
///
/// One store per source (ADR-0017), so the name has to separate two remotes of
/// the *same* kind: a WebDAV server and its sibling are both storage id `webdav`,
/// and only their roots differ. Which is why the identity is digested rather than
/// sanitised — a URL is not a file name.
pub fn store_name(id: &str, identity: &str) -> String {
    let digest = sha256_hex(identity.as_bytes());
    format!("{id}-{}", &digest[..8])
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex(&Sha256::digest(bytes))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> WebDavConfig {
        WebDavConfig {
            url: "http://localhost:8080".to_string(),
            path: String::new(),
            user: "u".to_string(),
            password: "p".to_string(),
        }
    }

    #[test]
    fn a_store_name_separates_two_remotes_of_one_kind() {
        let other = WebDavConfig {
            url: "http://localhost:8081".to_string(),
            ..config()
        };
        let a = store_name("webdav", &db_url(&config()));
        let b = store_name("webdav", &db_url(&other));
        assert_ne!(a, b, "two WebDAV roots are two stores");
        assert_eq!(a, store_name("webdav", &db_url(&config())));
        assert!(a.starts_with("webdav-"), "the kind stays readable: {a}");
        // …and it is a name, not the URL.
        assert!(!a.contains('/'));
    }

    #[test]
    fn webdav_db_url_always_lands_in_the_apps_namespace_folder() {
        assert_eq!(
            db_url(&config()),
            "http://localhost:8080/aricma-todo/todos.db"
        );
        assert_eq!(dir_url(&config()), "http://localhost:8080/aricma-todo");
        let with_slash = WebDavConfig {
            url: "http://localhost:8080/".to_string(),
            ..config()
        };
        assert_eq!(
            db_url(&with_slash),
            "http://localhost:8080/aricma-todo/todos.db"
        );
    }

    #[test]
    fn webdav_path_is_a_prefix_before_the_namespace_folder() {
        let prefixed = WebDavConfig {
            path: "/some/path".to_string(),
            ..config()
        };
        assert_eq!(
            db_url(&prefixed),
            "http://localhost:8080/some/path/aricma-todo/todos.db"
        );
        // Slashes and whitespace on either half are tolerated.
        let messy = WebDavConfig {
            url: "http://localhost:8080/".to_string(),
            path: " /some/path/ ".to_string(),
            ..config()
        };
        assert_eq!(
            db_url(&messy),
            "http://localhost:8080/some/path/aricma-todo/todos.db"
        );
    }

    #[test]
    fn webdav_auth_is_basic_base64_of_user_colon_password() {
        assert_eq!(auth_header(&config()), "Basic dTpw");
    }

    fn s3() -> S3Config {
        S3Config {
            endpoint: "https://s3.example.com".to_string(),
            bucket: "aricma-todo-test".to_string(),
            path: String::new(),
            access_key: "AKIDEXAMPLE".to_string(),
            secret_key: "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY".to_string(),
        }
    }

    #[test]
    fn s3_object_url_and_path_are_path_style() {
        assert_eq!(
            object_url(&s3()),
            "https://s3.example.com/aricma-todo-test/aricma-todo/todos.db"
        );
        assert_eq!(object_path(&s3()), "/aricma-todo-test/aricma-todo/todos.db");
        let trailing = S3Config {
            endpoint: "https://s3.example.com/".to_string(),
            ..s3()
        };
        assert_eq!(
            object_url(&trailing),
            "https://s3.example.com/aricma-todo-test/aricma-todo/todos.db"
        );
    }

    #[test]
    fn s3_path_is_a_prefix_before_the_namespace_folder() {
        let prefixed = S3Config {
            path: "team".to_string(),
            ..s3()
        };
        assert_eq!(
            object_path(&prefixed),
            "/aricma-todo-test/team/aricma-todo/todos.db"
        );
        assert_eq!(
            object_url(&prefixed),
            "https://s3.example.com/aricma-todo-test/team/aricma-todo/todos.db"
        );
    }

    #[test]
    fn s3_host_is_the_authority() {
        assert_eq!(host_of("https://s3.example.com"), "s3.example.com");
        assert_eq!(host_of("http://127.0.0.1:9000/x"), "127.0.0.1:9000");
        assert_eq!(host_of("localhost:9000"), "localhost:9000");
    }

    #[test]
    fn sha256_matches_the_known_digests() {
        assert_eq!(sha256_hex(b""), EMPTY_SHA256);
        assert_eq!(
            sha256_hex(b"Welcome to Amazon S3."),
            "44ce7dd67c959e0d3524ffac1771dfbba87d2b6b4b4e99e42034a8b803f8b072"
        );
    }

    #[test]
    fn the_canonical_request_is_byte_exact() {
        // The classic SigV4 formatting trap: the header block's trailing
        // newline is the separator before SignedHeaders, not an extra blank
        // line. Pinned as a literal, because a stray "\n" here silently
        // changes the signature.
        let canonical = canonical_request(
            "GET",
            "/aricma-todo-test/todos.db",
            "s3.example.com",
            EMPTY_SHA256,
            "20130524T000000Z",
        );
        assert_eq!(
            canonical,
            "GET\n/aricma-todo-test/todos.db\n\n\
             host:s3.example.com\n\
             x-amz-content-sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855\n\
             x-amz-date:20130524T000000Z\n\n\
             host;x-amz-content-sha256;x-amz-date\n\
             e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn the_string_to_sign_carries_the_scope_and_the_request_hash() {
        let canonical =
            canonical_request("GET", "/b/todos.db", "h", EMPTY_SHA256, "20130524T000000Z");
        let to_sign = string_to_sign(&canonical, "20130524T000000Z");
        let mut lines = to_sign.lines();
        assert_eq!(lines.next(), Some(S3_ALGORITHM));
        assert_eq!(lines.next(), Some("20130524T000000Z"));
        assert_eq!(lines.next(), Some("20130524/us-east-1/s3/aws4_request"));
        assert_eq!(
            lines.next(),
            Some(sha256_hex(canonical.as_bytes()).as_str())
        );
    }

    #[test]
    fn signing_produces_the_headers_a_signed_request_needs() {
        let headers = sign_request(
            &s3(),
            "GET",
            "/aricma-todo-test/todos.db",
            EMPTY_SHA256,
            "20130524T000000Z",
        );
        let names: Vec<&str> = headers.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "Host",
                "x-amz-content-sha256",
                "x-amz-date",
                "Authorization"
            ]
        );
        assert_eq!(headers[0].1, "s3.example.com");
        assert_eq!(headers[1].1, EMPTY_SHA256);
        assert_eq!(headers[2].1, "20130524T000000Z");

        let auth = &headers[3].1;
        assert!(
            auth.starts_with(
                "AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20130524/us-east-1/s3/aws4_request, \
                 SignedHeaders=host;x-amz-content-sha256;x-amz-date, Signature="
                    .replace("                 ", "")
                    .as_str()
            ),
            "{auth}"
        );
        // 64 lowercase hex digits — the HMAC-SHA256 output.
        let signature = auth.rsplit("Signature=").next().unwrap();
        assert_eq!(signature.len(), 64, "{signature}");
        assert!(
            signature.chars().all(|c| c.is_ascii_hexdigit()),
            "{signature}"
        );
    }

    #[test]
    fn a_different_payload_or_method_changes_the_signature() {
        let sign = |method: &str, payload: &str| {
            sign_request(&s3(), method, "/b/todos.db", payload, "20130524T000000Z")
                .pop()
                .unwrap()
                .1
        };
        let get = sign("GET", EMPTY_SHA256);
        assert_ne!(get, sign("PUT", EMPTY_SHA256), "method must be covered");
        assert_ne!(
            get,
            sign("GET", &sha256_hex(b"body")),
            "payload must be covered"
        );
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
