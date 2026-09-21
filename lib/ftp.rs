//! FTP as a **host-side** remote transport.
//!
//! A browser cannot open a TCP socket, and it has spoken no FTP since Chrome 86 /
//! Firefox 90 removed it — so, unlike WebDAV and S3, this transport cannot be
//! reached from the wasm app. It exists for a future TUI, CLI or standalone host,
//! and the module is gated to non-wasm builds so the browser bundle never sees it.
//! Nothing in `app/` offers it as a data source, on purpose: a tile the browser
//! could never connect to would be a lie.
//!
//! It mirrors the other transports' surface (`ensure` / `version` / `download` /
//! `upload`, all yielding a [`RemoteFile`]) so a future host can feed it to the
//! same reconcile. Plain FTP only: the URL must be `ftp://`, credentials go over
//! the wire in the clear, and the scheme check means a config cannot silently
//! downgrade. FTP also has no conditional write, so a caller gets the same
//! read-then-write sync guard Drive has, never a stronger one.
//!
//! ponytail: no FTPS (needs a TLS dependency), no active mode, no IPv6 literal
//! hosts, no `MLSD`. Add FTPS first when a real host consumes this.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::time::Duration;

use crate::remote::{RemoteFile, DB_FILE, SOURCE_DIR};

/// How long a control or data operation may block before it errors.
pub const TIMEOUT: Duration = Duration::from_secs(15);

/// An FTP remote: a root URL, an optional path prefix under it, and the
/// credentials. The same shape as the WebDAV config, minus the URL's path (that
/// is `path`) and minus TLS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FtpConfig {
    /// `ftp://host` or `ftp://host:port`. No path; port 21 when absent.
    pub url: String,
    /// Optional prefix under the login root, before the app's namespace folder
    /// (ADR-0019). Empty keeps the namespace directly under the login root.
    pub path: String,
    pub user: String,
    pub password: String,
}

/// The host and port [`FtpConfig::url`] names, defaulting to port 21. `ftps://`
/// (and anything else) is refused: this transport is plain FTP, and refusing the
/// scheme is what stops a config from claiming TLS it does not get.
pub fn host_port(config: &FtpConfig) -> Result<(String, u16), String> {
    let rest = config.url.trim();
    let rest = rest
        .strip_prefix("ftp://")
        .ok_or_else(|| format!("an FTP URL must start with ftp:// — got {:?}", config.url))?;
    let authority = rest.trim_end_matches('/').split('/').next().unwrap_or("");
    if authority.is_empty() {
        return Err("the FTP URL names no host".to_string());
    }
    match authority.rsplit_once(':') {
        Some((host, port)) if !host.is_empty() => {
            let port: u16 = port
                .parse()
                .map_err(|_| format!("bad FTP port {port:?} in {:?}", config.url))?;
            Ok((host.to_string(), port))
        }
        _ => Ok((authority.to_string(), 21)),
    }
}

/// The directory the database lives in: the configured prefix, then the app's
/// namespace folder (ADR-0019).
///
/// **Relative to the login root**, not absolute: an FTP session starts in the
/// user's home, and a leading `/` would instead mean the server's real filesystem
/// root on a server that does not chroot the user (vsftpd's default). Relative
/// works under chroot and without it.
pub fn db_dir(config: &FtpConfig) -> String {
    let prefix = config.path.trim().trim_matches('/');
    if prefix.is_empty() {
        SOURCE_DIR.to_string()
    } else {
        format!("{prefix}/{SOURCE_DIR}")
    }
}

/// The database's path on the server: [`db_dir`] plus the file name.
pub fn db_path(config: &FtpConfig) -> String {
    format!("{}/{DB_FILE}", db_dir(config))
}

/// The host and port in a `227 Entering Passive Mode (h1,h2,h3,h4,p1,p2)` reply.
pub fn pasv_addr(reply: &str) -> Option<(String, u16)> {
    let open = reply.find('(')?;
    let close = open + reply[open..].find(')')?;
    let parts: Vec<&str> = reply[open + 1..close].split(',').map(str::trim).collect();
    if parts.len() != 6 {
        return None;
    }
    let mut nums = [0u16; 6];
    for (slot, part) in nums.iter_mut().zip(parts) {
        *slot = part.parse().ok()?;
    }
    Some((
        format!("{}.{}.{}.{}", nums[0], nums[1], nums[2], nums[3]),
        nums[4] * 256 + nums[5],
    ))
}

/// A control connection to an FTP server. One session per client; the caller
/// opens it, does its file work, and drops it.
pub struct FtpClient {
    write: TcpStream,
    read: BufReader<TcpStream>,
}

impl FtpClient {
    /// Connect, sign in, and switch to binary mode.
    pub fn connect(config: &FtpConfig) -> Result<FtpClient, String> {
        let (host, port) = host_port(config)?;
        let write = TcpStream::connect((host.as_str(), port))
            .map_err(|e| format!("FTP connect to {host}:{port} failed: {e}"))?;
        write
            .set_read_timeout(Some(TIMEOUT))
            .map_err(|e| e.to_string())?;
        write
            .set_write_timeout(Some(TIMEOUT))
            .map_err(|e| e.to_string())?;
        let read = BufReader::new(write.try_clone().map_err(|e| e.to_string())?);
        let mut client = FtpClient { write, read };

        let (code, text) = client.read_reply()?;
        if code != 220 {
            return Err(format!("the FTP server greeted with {code}: {text}"));
        }
        // 230 to USER alone means the server does not want a password.
        let (code, _) = client.command("USER", &config.user, &[331, 230])?;
        if code == 331 {
            client.command("PASS", &config.password, &[230, 202])?;
        }
        client.command("TYPE", "I", &[200])?;
        Ok(client)
    }

    /// The remote database, or `None` when the server has none yet.
    pub fn ensure(&mut self, config: &FtpConfig) -> Result<Option<RemoteFile>, String> {
        let path = db_path(config);
        match self.size(&path)? {
            None => Ok(None),
            Some(_) => {
                let version = self.mdtm(&path)?;
                Ok(Some(RemoteFile { id: path, version }))
            }
        }
    }

    /// A fresh version token. FTP has no `ETag`; this is the file's `MDTM` — a
    /// weak, second-granularity timestamp, so it identifies the remote but guards
    /// nothing.
    pub fn version(&mut self, config: &FtpConfig) -> Result<String, String> {
        let path = db_path(config);
        self.mdtm(&path)?
            .ok_or_else(|| "the FTP server reported no modification time".to_string())
    }

    /// Download the database bytes.
    pub fn download(&mut self, config: &FtpConfig) -> Result<Vec<u8>, String> {
        self.retrieve(&db_path(config))
    }

    /// Upload the database, returning the remote file with its new version.
    pub fn upload(&mut self, config: &FtpConfig, bytes: &[u8]) -> Result<RemoteFile, String> {
        let path = db_path(config);
        self.store(&path, bytes)?;
        let version = self.mdtm(&path)?;
        Ok(RemoteFile { id: path, version })
    }

    /// Remove the remote database.
    pub fn delete(&mut self, config: &FtpConfig) -> Result<(), String> {
        let path = db_path(config);
        let (code, text) = self.command("DELE", &path, &[250, 200, 550])?;
        if code == 550 {
            return Err(format!("FTP DELE failed: {text}"));
        }
        Ok(())
    }

    /// Create the namespace folder the database lives in, and any missing prefix
    /// above it. Paths are relative to the login root (see [`db_dir`]).
    pub fn ensure_namespace(&mut self, config: &FtpConfig) -> Result<(), String> {
        let mut path = String::new();
        for part in db_dir(config).split('/').filter(|part| !part.is_empty()) {
            if !path.is_empty() {
                path.push('/');
            }
            path.push_str(part);
            // 550/521 is "already there" on vsftpd; FTP has no other code for it.
            // ponytail: this also swallows a permission error — split them if a
            // host ever needs to tell the two apart.
            self.command("MKD", &path, &[257, 550, 521])?;
        }
        Ok(())
    }

    // ── protocol ─────────────────────────────────────────────────────────

    /// Send a command and return its reply, erroring on an unexpected code.
    fn command(
        &mut self,
        verb: &str,
        arg: &str,
        expected: &[u16],
    ) -> Result<(u16, String), String> {
        self.send(verb, arg)?;
        let (code, text) = self.read_reply()?;
        if expected.contains(&code) {
            Ok((code, text))
        } else {
            Err(format!("FTP {verb} failed ({code}): {text}"))
        }
    }

    /// Send a command without reading its reply — transfer commands reply twice
    /// (a preliminary 150, then 226), so the caller reads both.
    fn send(&mut self, verb: &str, arg: &str) -> Result<(), String> {
        let line = if arg.is_empty() {
            format!("{verb}\r\n")
        } else {
            format!("{verb} {arg}\r\n")
        };
        self.write
            .write_all(line.as_bytes())
            .map_err(|e| format!("FTP write failed: {e}"))
    }

    /// One CRLF-terminated, trimmed reply line, or an error when the connection
    /// closes.
    fn read_line(&mut self) -> Result<String, String> {
        let mut line = String::new();
        let n = self
            .read
            .read_line(&mut line)
            .map_err(|e| format!("FTP read failed: {e}"))?;
        if n == 0 {
            return Err("the FTP server closed the connection".to_string());
        }
        Ok(line.trim_end_matches(['\r', '\n']).to_string())
    }

    /// Read one reply, folding a multi-line reply (`123-…` … `123 done`) into its
    /// text.
    fn read_reply(&mut self) -> Result<(u16, String), String> {
        let first = self.read_line()?;
        let code: u16 = first
            .get(..3)
            .and_then(|code| code.parse().ok())
            .ok_or_else(|| format!("malformed FTP reply: {first:?}"))?;
        let mut text = first.clone();
        if first.as_bytes().get(3) == Some(&b'-') {
            let terminator = format!("{code} ");
            loop {
                let cont = self.read_line()?;
                text.push('\n');
                text.push_str(&cont);
                if cont.starts_with(&terminator) {
                    break;
                }
            }
        }
        Ok((code, text))
    }

    /// The `SIZE` of a path, or `None` when the server answers 550 (absent).
    fn size(&mut self, path: &str) -> Result<Option<u64>, String> {
        let (code, text) = self.command("SIZE", path, &[213, 550])?;
        if code == 550 {
            return Ok(None);
        }
        Ok(text.split_whitespace().last().and_then(|n| n.parse().ok()))
    }

    /// The `MDTM` of a path, or `None` when the server answers 550.
    fn mdtm(&mut self, path: &str) -> Result<Option<String>, String> {
        let (code, text) = self.command("MDTM", path, &[213, 550])?;
        if code == 550 {
            return Ok(None);
        }
        Ok(text.split_whitespace().last().map(str::to_string))
    }

    /// Enter passive mode and connect the data channel it named.
    fn passive(&mut self) -> Result<TcpStream, String> {
        let (_, text) = self.command("PASV", "", &[227])?;
        let (host, port) =
            pasv_addr(&text).ok_or_else(|| format!("malformed PASV reply: {text:?}"))?;
        let data = TcpStream::connect((host.as_str(), port))
            .map_err(|e| format!("FTP data connect to {host}:{port} failed: {e}"))?;
        data.set_read_timeout(Some(TIMEOUT))
            .map_err(|e| e.to_string())?;
        data.set_write_timeout(Some(TIMEOUT))
            .map_err(|e| e.to_string())?;
        Ok(data)
    }

    /// `RETR` a path over the data channel.
    fn retrieve(&mut self, path: &str) -> Result<Vec<u8>, String> {
        let mut data = self.passive()?;
        self.send("RETR", path)?;
        let (code, text) = self.read_reply()?;
        if code != 150 && code != 125 {
            return Err(format!("FTP RETR failed ({code}): {text}"));
        }
        let mut bytes = Vec::new();
        data.read_to_end(&mut bytes)
            .map_err(|e| format!("FTP data read failed: {e}"))?;
        drop(data);
        let (code, text) = self.read_reply()?;
        if !(200..300).contains(&code) {
            return Err(format!("FTP RETR did not complete ({code}): {text}"));
        }
        Ok(bytes)
    }

    /// `STOR` bytes to a path over the data channel.
    fn store(&mut self, path: &str, bytes: &[u8]) -> Result<(), String> {
        let mut data = self.passive()?;
        self.send("STOR", path)?;
        let (code, text) = self.read_reply()?;
        if code != 150 && code != 125 {
            return Err(format!("FTP STOR failed ({code}): {text}"));
        }
        data.write_all(bytes)
            .map_err(|e| format!("FTP data write failed: {e}"))?;
        let _ = data.shutdown(Shutdown::Write);
        drop(data);
        let (code, text) = self.read_reply()?;
        if !(200..300).contains(&code) {
            return Err(format!("FTP STOR did not complete ({code}): {text}"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(url: &str, path: &str) -> FtpConfig {
        FtpConfig {
            url: url.to_string(),
            path: path.to_string(),
            user: "u".to_string(),
            password: "p".to_string(),
        }
    }

    #[test]
    fn host_port_defaults_to_21_and_reads_an_explicit_port() {
        assert_eq!(
            host_port(&config("ftp://example.com", "")).unwrap(),
            ("example.com".to_string(), 21)
        );
        assert_eq!(
            host_port(&config("ftp://example.com:2121/", "")).unwrap(),
            ("example.com".to_string(), 2121)
        );
    }

    #[test]
    fn a_url_that_is_not_plain_ftp_is_refused() {
        assert!(host_port(&config("ftps://example.com", "")).is_err());
        assert!(host_port(&config("https://example.com", "")).is_err());
        assert!(host_port(&config("", "")).is_err());
    }

    #[test]
    fn the_db_path_lands_in_the_namespace_folder() {
        assert_eq!(db_path(&config("ftp://h", "")), "aricma-todo/todos.db");
        assert_eq!(
            db_path(&config("ftp://h", "/some/path/")),
            "some/path/aricma-todo/todos.db"
        );
        assert_eq!(db_dir(&config("ftp://h", "team")), "team/aricma-todo");
    }

    #[test]
    fn a_pasv_reply_yields_host_and_port() {
        assert_eq!(
            pasv_addr("227 Entering Passive Mode (127,0,0,1,82,14)."),
            Some(("127.0.0.1".to_string(), 82 * 256 + 14))
        );
        assert_eq!(pasv_addr("227 broken"), None);
        assert_eq!(pasv_addr("227 (1,2,3,4,5)."), None);
    }

    #[test]
    fn connect_refuses_a_bad_password() {
        let server = fake::FakeFtp::start("u", "p");
        let mut bad = config(&format!("ftp://127.0.0.1:{}", server.port), "");
        bad.password = "wrong".to_string();
        assert!(FtpClient::connect(&bad).is_err());
    }

    #[test]
    fn ensure_is_none_before_the_first_upload() {
        let server = fake::FakeFtp::start("u", "p");
        let cfg = config(&format!("ftp://127.0.0.1:{}", server.port), "");
        let mut client = FtpClient::connect(&cfg).unwrap();
        assert_eq!(client.ensure(&cfg).unwrap(), None);
    }

    #[test]
    fn upload_then_ensure_and_download_round_trip() {
        let server = fake::FakeFtp::start("u", "p");
        let cfg = config(&format!("ftp://127.0.0.1:{}", server.port), "team");
        let mut client = FtpClient::connect(&cfg).unwrap();
        client.ensure_namespace(&cfg).unwrap();

        let stored = client.upload(&cfg, b"the bytes").unwrap();
        assert_eq!(stored.id, "team/aricma-todo/todos.db");
        assert_eq!(stored.version, Some("20260921000000".to_string()));

        let found = client.ensure(&cfg).unwrap().unwrap();
        assert_eq!(found.id, db_path(&cfg));
        assert_eq!(client.version(&cfg).unwrap(), "20260921000000");
        assert_eq!(client.download(&cfg).unwrap(), b"the bytes");
    }

    #[test]
    fn delete_removes_the_remote_file() {
        let server = fake::FakeFtp::start("u", "p");
        let cfg = config(&format!("ftp://127.0.0.1:{}", server.port), "");
        let mut client = FtpClient::connect(&cfg).unwrap();
        client.ensure_namespace(&cfg).unwrap();
        client.upload(&cfg, b"x").unwrap();
        assert!(client.ensure(&cfg).unwrap().is_some());

        client.delete(&cfg).unwrap();
        assert_eq!(client.ensure(&cfg).unwrap(), None);
    }

    /// The real server, in a container. `#[ignore]`d so plain `cargo test` stays
    /// hermetic and needs no Docker; run it with
    /// `docker compose up -d ftp && cargo test --lib ftp -- --ignored`. It lives
    /// in the lib rather than `tests/` because an integration-test target makes
    /// cargo build the package's **bins**, and the app bin is wasm-only.
    #[test]
    #[ignore = "needs the ftp container: docker compose up -d ftp"]
    fn ftp_round_trips_against_vsftpd() {
        let env =
            |key: &str, default: &str| std::env::var(key).unwrap_or_else(|_| default.to_string());
        let cfg = FtpConfig {
            url: env("ARICMA_FTP_URL", "ftp://127.0.0.1:2121"),
            // A per-run prefix under the login root, so a rerun starts clean and
            // `MKD` proves the namespace folder and its prefix are created.
            path: format!("it-{}", std::process::id()),
            user: env("ARICMA_FTP_USER", "aricma-test"),
            password: env("ARICMA_FTP_PASSWORD", "aricma-test-secret"),
        };

        let mut client = FtpClient::connect(&cfg).expect("connect");
        assert_eq!(client.ensure(&cfg).expect("ensure"), None);

        client.ensure_namespace(&cfg).expect("mkdir namespace");
        let bytes = b"aricma ftp integration\n";
        let stored = client.upload(&cfg, bytes).expect("upload");
        assert!(stored.version.is_some(), "vsftpd reports an MDTM");

        assert_eq!(
            client.ensure(&cfg).expect("ensure").unwrap().id,
            db_path(&cfg)
        );
        assert_eq!(client.download(&cfg).expect("download"), bytes);

        client.delete(&cfg).expect("delete");
        assert_eq!(client.ensure(&cfg).expect("ensure"), None);
    }
}

/// An in-process vsftpd stand-in: enough of the protocol (control, PASV, RETR,
/// STOR, SIZE, MDTM, DELE, MKD) that the client is exercised over real sockets
/// with no Docker. Its job is the client's protocol logic, not vsftpd's.
#[cfg(test)]
mod fake {
    use std::collections::BTreeMap;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::{Arc, Mutex};
    use std::thread;

    type Files = Arc<Mutex<BTreeMap<String, Vec<u8>>>>;

    pub struct FakeFtp {
        pub port: u16,
        #[allow(dead_code)]
        files: Files,
    }

    impl FakeFtp {
        pub fn start(user: &str, password: &str) -> FakeFtp {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake ftp");
            let port = listener.local_addr().unwrap().port();
            let files: Files = Arc::new(Mutex::new(BTreeMap::new()));
            let state = files.clone();
            let user = user.to_string();
            let password = password.to_string();
            thread::spawn(move || {
                for stream in listener.incoming() {
                    let Ok(stream) = stream else { break };
                    let state = state.clone();
                    let user = user.clone();
                    let password = password.clone();
                    thread::spawn(move || {
                        let _ = serve(stream, state, &user, &password);
                    });
                }
            });
            FakeFtp { port, files }
        }
    }

    fn serve(stream: TcpStream, files: Files, user: &str, password: &str) -> std::io::Result<()> {
        let mut write = stream.try_clone()?;
        let mut read = BufReader::new(stream);
        write.write_all(b"220 fake ready\r\n")?;
        let mut logged_in = false;
        let mut data: Option<TcpListener> = None;
        let mut line = String::new();
        loop {
            line.clear();
            if read.read_line(&mut line)? == 0 {
                return Ok(());
            }
            let line = line.trim_end_matches(['\r', '\n']);
            let (verb, arg) = match line.split_once(' ') {
                Some((verb, arg)) => (verb.to_ascii_uppercase(), arg.to_string()),
                None => (line.to_ascii_uppercase(), String::new()),
            };
            if !logged_in && !matches!(verb.as_str(), "USER" | "PASS" | "QUIT") {
                write.write_all(b"530 not logged in\r\n")?;
                continue;
            }
            match verb.as_str() {
                "USER" => {
                    if arg == user {
                        write.write_all(b"331 need password\r\n")?;
                    } else {
                        write.write_all(b"530 no such user\r\n")?;
                    }
                }
                "PASS" => {
                    if arg == password {
                        logged_in = true;
                        write.write_all(b"230 logged in\r\n")?;
                    } else {
                        write.write_all(b"530 login incorrect\r\n")?;
                    }
                }
                "TYPE" => write.write_all(b"200 type set\r\n")?,
                "PASV" => {
                    let listener = TcpListener::bind("127.0.0.1:0")?;
                    let port = listener.local_addr()?.port();
                    data = Some(listener);
                    write.write_all(
                        format!(
                            "227 Entering Passive Mode (127,0,0,1,{},{}).\r\n",
                            port / 256,
                            port % 256
                        )
                        .as_bytes(),
                    )?;
                }
                "SIZE" => match files.lock().unwrap().get(&arg) {
                    Some(bytes) => {
                        write.write_all(format!("213 {}\r\n", bytes.len()).as_bytes())?
                    }
                    None => write.write_all(b"550 not found\r\n")?,
                },
                "MDTM" => {
                    if files.lock().unwrap().contains_key(&arg) {
                        write.write_all(b"213 20260921000000\r\n")?;
                    } else {
                        write.write_all(b"550 not found\r\n")?;
                    }
                }
                "RETR" => {
                    let bytes = files.lock().unwrap().get(&arg).cloned();
                    match (bytes, data.take()) {
                        (Some(bytes), Some(listener)) => {
                            write.write_all(b"150 opening data\r\n")?;
                            let (mut conn, _) = listener.accept()?;
                            conn.write_all(&bytes)?;
                            drop(conn);
                            write.write_all(b"226 transfer complete\r\n")?;
                        }
                        _ => write.write_all(b"550 not found\r\n")?,
                    }
                }
                "STOR" => match data.take() {
                    Some(listener) => {
                        write.write_all(b"150 ok to send\r\n")?;
                        let (mut conn, _) = listener.accept()?;
                        let mut bytes = Vec::new();
                        conn.read_to_end(&mut bytes)?;
                        files.lock().unwrap().insert(arg.clone(), bytes);
                        drop(conn);
                        write.write_all(b"226 stored\r\n")?;
                    }
                    None => write.write_all(b"425 no data connection\r\n")?,
                },
                "DELE" => {
                    if files.lock().unwrap().remove(&arg).is_some() {
                        write.write_all(b"250 deleted\r\n")?;
                    } else {
                        write.write_all(b"550 not found\r\n")?;
                    }
                }
                "MKD" => write.write_all(b"257 created\r\n")?,
                "QUIT" => {
                    write.write_all(b"221 bye\r\n")?;
                    return Ok(());
                }
                _ => write.write_all(b"502 not implemented\r\n")?,
            }
        }
    }
}
