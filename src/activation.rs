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

const REQUEST: &str = "ACTIVATE 1\n";
const RESPONSE_OK: &str = "OK REQUESTED\n";
const RESPONSE_BAD_REQUEST: &str = "ERR BAD_REQUEST\n";
const RESPONSE_UNAUTHORIZED: &str = "ERR UNAUTHORIZED\n";
const RESPONSE_INTERNAL: &str = "ERR INTERNAL\n";

pub struct ActivationServer {
    path: PathBuf,
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl ActivationServer {
    pub fn start(paths: &XdgPaths, ctx: egui::Context, tx: Sender<()>) -> Result<Self, String> {
        Self::start_at(paths.activation_socket_path(), ctx, tx)
    }

    pub fn start_at(path: PathBuf, ctx: egui::Context, tx: Sender<()>) -> Result<Self, String> {
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
                        let mut buffer = [0u8; 64];
                        let read = match stream.read(&mut buffer) {
                            Ok(read) => read,
                            Err(err) => {
                                crate::logging::log_warn(
                                    "activation",
                                    format!("Aktivierungsanfrage nicht lesbar: {err}"),
                                );
                                let _ = stream.write_all(RESPONSE_BAD_REQUEST.as_bytes());
                                continue;
                            }
                        };
                        let request = String::from_utf8_lossy(&buffer[..read]);
                        if request.trim() != REQUEST.trim() {
                            let _ = stream.write_all(RESPONSE_BAD_REQUEST.as_bytes());
                            continue;
                        }
                        if tx.send(()).is_err() {
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
    Accepted,
    NotRunning,
    Unavailable(String),
}

impl ActivationResult {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Accepted)
    }

    pub fn user_message(&self) -> String {
        match self {
            Self::Accepted => {
                "Aktivierungsanforderung übermittelt (Fensterfokus nicht bestätigt).".to_owned()
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
    send_activation_at(&paths.activation_socket_path(), &paths.instance_lock_path())
}

fn send_activation_at(socket_path: &Path, lock_path: &Path) -> ActivationResult {
    match send_to_path(socket_path) {
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

fn send_to_path(path: &Path) -> Result<ActivationResult, String> {
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
        .write_all(REQUEST.as_bytes())
        .map_err(|err| format!("Aktivierungsanfrage konnte nicht gesendet werden: {err}"))?;
    let mut response = [0u8; 64];
    let read = stream
        .read(&mut response)
        .map_err(|err| format!("Aktivierungsantwort konnte nicht gelesen werden: {err}"))?;
    let response = String::from_utf8_lossy(&response[..read]);
    if response.trim() == RESPONSE_OK.trim() {
        Ok(ActivationResult::Accepted)
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
        let result = send_to_path(&path).unwrap();
        assert_eq!(result, ActivationResult::Accepted);
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
        let result = send_to_path(&path).unwrap();
        assert_eq!(result, ActivationResult::Accepted);
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
        let result = send_activation_at(&socket, &lock);
        assert_eq!(result, ActivationResult::NotRunning);
        assert!(!socket.exists());
    }
}
