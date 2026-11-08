//! Local IPC used by `couchdeck --activate`.
//!
//! The running UI owns a Unix-domain socket in the per-user XDG runtime
//! directory (or the state directory as fallback). The socket is mode 0600 and
//! the server additionally verifies `SO_PEERCRED`; another local user cannot
//! send activation requests. The server never touches game processes and only
//! asks the eframe UI to perform its normal activation path.

use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::io::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use eframe::egui;

use crate::instance::{InstanceError, InstanceGuard};
use crate::paths::XdgPaths;

const REQUEST_HEADER: &str = "ACTIVATE 2\n";
const REQUEST_END: &str = "END\n";
const MAX_REQUEST_BYTES: usize = 16 * 1024;
const MAX_TOKEN_BYTES: usize = 4 * 1024;
const RESPONSE_OK: &str = "OK REQUESTED\n";
const RESPONSE_BAD_REQUEST: &str = "ERR BAD_REQUEST\n";
const RESPONSE_UNAUTHORIZED: &str = "ERR UNAUTHORIZED\n";
const RESPONSE_INTERNAL: &str = "ERR INTERNAL\n";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivationRequest {
    /// Optional xdg-activation-v1 token forwarded by the CLI caller.
    pub token: Option<String>,
}

pub struct ActivationServer {
    path: PathBuf,
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl ActivationServer {
    pub fn start(
        paths: &XdgPaths,
        ctx: egui::Context,
        tx: Sender<ActivationRequest>,
    ) -> Result<Self, String> {
        Self::start_at(paths.activation_socket_path(), ctx, tx)
    }

    pub fn start_at(
        path: PathBuf,
        ctx: egui::Context,
        tx: Sender<ActivationRequest>,
    ) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                format!(
                    "Aktivierungs-Socket-Verzeichnis {} kann nicht erstellt werden: {err}",
                    parent.display()
                )
            })?;
        }

        let listener = bind_listener(&path)?;
        #[cfg(unix)]
        {
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).map_err(|err| {
                format!(
                    "Aktivierungs-Socket {} konnte nicht auf 0600 gesetzt werden: {err}",
                    path.display()
                )
            })?;
        }
        listener.set_nonblocking(true).map_err(|err| {
            format!("Aktivierungs-Socket kann nicht non-blocking gesetzt werden: {err}")
        })?;

        let stop = Arc::new(AtomicBool::new(false));
        let stop_for_thread = stop.clone();
        let path_for_thread = path.clone();
        let handle = thread::spawn(move || {
            while !stop_for_thread.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _addr)) => {
                        let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
                        let _ = stream.set_write_timeout(Some(Duration::from_millis(500)));
                        if !peer_is_same_user(&stream) {
                            crate::logging::log_warn(
                                "activation",
                                "Aktivierungsanfrage von anderem lokalen Benutzer abgelehnt.",
                            );
                            let _ = stream.write_all(RESPONSE_UNAUTHORIZED.as_bytes());
                            continue;
                        }
                        let request = match read_request(&mut stream) {
                            Ok(request) => request,
                            Err(err) => {
                                crate::logging::log_warn(
                                    "activation",
                                    format!("Aktivierungsanfrage nicht lesbar: {err}"),
                                );
                                let _ = stream.write_all(RESPONSE_BAD_REQUEST.as_bytes());
                                continue;
                            }
                        };
                        crate::logging::log_info(
                            "activation",
                            format!(
                                "Anfrage vom lokalen Benutzer empfangen; Token {}.",
                                if request.token.is_some() {
                                    "vorhanden"
                                } else {
                                    "nicht vorhanden"
                                }
                            ),
                        );
                        if tx.send(request).is_err() {
                            let _ = stream.write_all(RESPONSE_INTERNAL.as_bytes());
                            break;
                        }
                        ctx.request_repaint();
                        let _ = stream.write_all(RESPONSE_OK.as_bytes());
                        crate::logging::log_info(
                            "activation",
                            "Aktivierungsanforderung an die UI übermittelt.",
                        );
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(100));
                    }
                    Err(err) => {
                        crate::logging::log_warn(
                            "activation",
                            format!("Aktivierungs-Socket {}: {err}", path_for_thread.display()),
                        );
                        thread::sleep(Duration::from_millis(200));
                    }
                }
            }
        });

        Ok(Self {
            path,
            stop,
            handle: Some(handle),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ActivationServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        let _ = fs::remove_file(&self.path);
    }
}

fn bind_listener(path: &Path) -> Result<UnixListener, String> {
    match UnixListener::bind(path) {
        Ok(listener) => Ok(listener),
        Err(err) if err.kind() == std::io::ErrorKind::AddrInUse => {
            // A connectable socket means another server is alive. Otherwise a
            // previous CouchDeck process crashed and left a stale socket.
            if UnixStream::connect(path).is_ok() {
                return Err(format!(
                    "Aktivierungs-Socket {} ist bereits von einer laufenden Instanz belegt.",
                    path.display()
                ));
            }
            fs::remove_file(path).map_err(|remove_err| {
                format!(
                    "Veralteter Aktivierungs-Socket {} kann nicht entfernt werden: {remove_err}",
                    path.display()
                )
            })?;
            UnixListener::bind(path)
                .map_err(|bind_err| format!("Aktivierungs-Socket {}: {bind_err}", path.display()))
        }
        Err(err) => Err(format!("Aktivierungs-Socket {}: {err}", path.display())),
    }
}

fn read_request(stream: &mut UnixStream) -> Result<ActivationRequest, String> {
    let mut text = String::new();
    let mut chunk = [0u8; 512];
    loop {
        let read = stream
            .read(&mut chunk)
            .map_err(|err| format!("Lesefehler: {err}"))?;
        if read == 0 {
            break;
        }
        text.push_str(&String::from_utf8_lossy(&chunk[..read]));
        if text.len() > MAX_REQUEST_BYTES {
            return Err("Anfrage zu groß".to_owned());
        }
        if text
            .lines()
            .any(|line| line.trim_end_matches('\r') == "END")
        {
            break;
        }
    }

    let mut lines = text.lines().map(|line| line.trim_end_matches('\r'));
    let header = lines.next().unwrap_or_default();
    match header {
        "ACTIVATE 1" => Ok(ActivationRequest { token: None }),
        "ACTIVATE 2" => {
            let mut token = None;
            let mut saw_end = false;
            for line in lines {
                if line == "END" {
                    saw_end = true;
                    break;
                }
                if let Some(encoded) = line.strip_prefix("TOKEN ") {
                    if encoded.len() > MAX_TOKEN_BYTES * 2 {
                        return Err("Activation-Token zu groß".to_owned());
                    }
                    let decoded = hex_decode(encoded)?;
                    if decoded.len() > MAX_TOKEN_BYTES {
                        return Err("Activation-Token zu groß".to_owned());
                    }
                    token = Some(decoded);
                }
            }
            if !saw_end {
                return Err("Anfrage ohne END".to_owned());
            }
            Ok(ActivationRequest { token })
        }
        _ => Err("Unbekannte Aktivierungsprotokollversion".to_owned()),
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn hex_decode(value: &str) -> Result<String, String> {
    if value.len() % 2 != 0 {
        return Err("Ungültiger hexadezimaler Activation-Token".to_owned());
    }
    let mut bytes = Vec::with_capacity(value.len() / 2);
    let raw = value.as_bytes();
    for pair in raw.chunks_exact(2) {
        let high = hex_nibble(pair[0])?;
        let low = hex_nibble(pair[1])?;
        bytes.push((high << 4) | low);
    }
    String::from_utf8(bytes).map_err(|_| "Activation-Token ist nicht UTF-8".to_owned())
}

fn hex_nibble(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err("Ungültiger hexadezimaler Activation-Token".to_owned()),
    }
}

fn peer_is_same_user(stream: &UnixStream) -> bool {
    let mut cred = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    let rc = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            &mut cred as *mut _ as *mut libc::c_void,
            &mut len,
        )
    };
    if rc != 0 {
        return false;
    }
    cred.uid == unsafe { libc::geteuid() }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivationResult {
    /// The running UI accepted the request. This does not prove that the
    /// compositor granted focus.
    Accepted {
        token_forwarded: bool,
    },
    NotRunning,
    Unavailable(String),
}

impl ActivationResult {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Accepted { .. })
    }

    pub fn user_message(&self) -> String {
        match self {
            Self::Accepted { token_forwarded } => {
                if *token_forwarded {
                    "Aktivierungsanforderung übermittelt (Activation-Token weitergegeben; Fensterfokus nicht bestätigt).".to_owned()
                } else {
                    "Aktivierungsanforderung übermittelt (kein Activation-Token vorhanden; Fensterfokus nicht bestätigt).".to_owned()
                }
            }
            Self::NotRunning => {
                "CouchDeck läuft nicht; es gibt keine Instanz zum Aktivieren.".to_owned()
            }
            Self::Unavailable(reason) => {
                format!("CouchDeck läuft, Aktivierung nicht möglich: {reason}")
            }
        }
    }
}

pub fn send_activation(paths: &XdgPaths) -> ActivationResult {
    send_activation_at(
        &paths.activation_socket_path(),
        &paths.instance_lock_path(),
        read_activation_token().as_deref(),
    )
}

pub fn read_activation_token() -> Option<String> {
    std::env::var("XDG_ACTIVATION_TOKEN")
        .ok()
        .map(|token| token.trim().to_owned())
        .filter(|token| !token.is_empty())
}

fn send_activation_at(
    socket_path: &Path,
    lock_path: &Path,
    token: Option<&str>,
) -> ActivationResult {
    match send_to_path(socket_path, token) {
        Ok(result) => result,
        Err(connect_error) => {
            // Distinguish a genuinely absent instance from a live UI whose IPC
            // endpoint is not ready/broken. The instance lock is authoritative.
            let running = match InstanceGuard::acquire(lock_path) {
                Ok(guard) => {
                    drop(guard);
                    false
                }
                Err(InstanceError::AlreadyRunning) => true,
                Err(_) => true,
            };
            if running {
                ActivationResult::Unavailable(connect_error)
            } else {
                let _ = fs::remove_file(socket_path);
                ActivationResult::NotRunning
            }
        }
    }
}

fn send_to_path(path: &Path, token: Option<&str>) -> Result<ActivationResult, String> {
    let mut request = String::from(REQUEST_HEADER);
    if let Some(token) = token {
        request.push_str("TOKEN ");
        request.push_str(&hex_encode(token.as_bytes()));
        request.push('\n');
    }
    request.push_str(REQUEST_END);

    let mut stream = UnixStream::connect(path).map_err(|err| match err.kind() {
        std::io::ErrorKind::NotFound => "Aktivierungs-Socket nicht gefunden.".to_owned(),
        std::io::ErrorKind::ConnectionRefused => {
            "Aktivierungs-Socket verweigert die Verbindung.".to_owned()
        }
        _ => format!("Aktivierungs-Socket {}: {err}", path.display()),
    })?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    stream
        .write_all(request.as_bytes())
        .map_err(|err| format!("Aktivierungsanfrage konnte nicht gesendet werden: {err}"))?;
    let mut response = [0u8; 64];
    let read = stream
        .read(&mut response)
        .map_err(|err| format!("Aktivierungsantwort konnte nicht gelesen werden: {err}"))?;
    let response = String::from_utf8_lossy(&response[..read]);
    if response.trim() == RESPONSE_OK.trim() {
        Ok(ActivationResult::Accepted {
            token_forwarded: token.is_some(),
        })
    } else if response.trim() == RESPONSE_UNAUTHORIZED.trim() {
        Ok(ActivationResult::Unavailable(
            "Zugriff verweigert (anderer Benutzer).".to_owned(),
        ))
    } else {
        Ok(ActivationResult::Unavailable(format!(
            "Unerwartete Antwort der laufenden Instanz: {}",
            response.trim()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;

    fn temp_socket() -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("activate.sock");
        (tmp, path)
    }

    #[test]
    fn activation_server_accepts_requests_and_cleans_socket() {
        let (_tmp, path) = temp_socket();
        let ctx = egui::Context::default();
        let (tx, rx) = std::sync::mpsc::channel();
        let server = ActivationServer::start_at(path.clone(), ctx, tx).unwrap();
        use std::os::unix::fs::MetadataExt;
        assert_eq!(std::fs::metadata(&path).unwrap().mode() & 0o777, 0o600);
        let result = send_to_path(&path, None).unwrap();
        assert_eq!(
            result,
            ActivationResult::Accepted {
                token_forwarded: false
            }
        );
        rx.recv_timeout(Duration::from_secs(2)).unwrap();
        drop(server);
        assert!(!path.exists());
    }

    #[test]
    fn stale_socket_is_replaced() {
        let (_tmp, path) = temp_socket();
        let listener = UnixListener::bind(&path).unwrap();
        drop(listener);
        let ctx = egui::Context::default();
        let (tx, rx) = std::sync::mpsc::channel();
        let server = ActivationServer::start_at(path.clone(), ctx, tx).unwrap();
        let result = send_to_path(&path, None).unwrap();
        assert_eq!(
            result,
            ActivationResult::Accepted {
                token_forwarded: false
            }
        );
        rx.recv_timeout(Duration::from_secs(2)).unwrap();
        drop(server);
    }

    #[test]
    fn activation_without_instance_reports_not_running_and_removes_stale_socket() {
        let tmp = tempfile::tempdir().unwrap();
        let socket = tmp.path().join("activate.sock");
        let lock = tmp.path().join("instance.lock");
        // Simulate a crashed process: the socket file exists but nobody listens.
        let stale = UnixListener::bind(&socket).unwrap();
        drop(stale);
        let result = send_activation_at(&socket, &lock, None);
        assert_eq!(result, ActivationResult::NotRunning);
        assert!(!socket.exists());
    }
    #[test]
    fn activation_token_is_forwarded_without_exposing_it_in_logs() {
        let (_tmp, path) = temp_socket();
        let ctx = egui::Context::default();
        let (tx, rx) = std::sync::mpsc::channel();
        let server = ActivationServer::start_at(path.clone(), ctx, tx).unwrap();
        let result = send_to_path(&path, Some("kwin-test-token")).unwrap();
        assert_eq!(
            result,
            ActivationResult::Accepted {
                token_forwarded: true
            }
        );
        let request = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(request.token.as_deref(), Some("kwin-test-token"));
        drop(server);
    }

    #[test]
    fn activation_token_hex_roundtrip_and_invalid_input() {
        let token = "kwin-123:token/with spaces";
        assert_eq!(hex_decode(&hex_encode(token.as_bytes())).unwrap(), token);
        assert!(hex_decode("abc").is_err());
        assert!(hex_decode("zz").is_err());
    }
}
