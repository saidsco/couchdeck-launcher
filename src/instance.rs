//! Per-user single-instance guard.
//!
//! An advisory `flock` on a per-user lock file prevents autostart and a manual
//! start from running two independent UIs against the same library and time
//! budget. The kernel releases the lock automatically if the process crashes.

use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use fs2::FileExt;

#[derive(Debug)]
pub enum InstanceError {
    AlreadyRunning,
    Io(String),
}

impl fmt::Display for InstanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyRunning => write!(f, "CouchDeck läuft bereits für diesen Benutzer."),
            Self::Io(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for InstanceError {}

pub struct InstanceGuard {
    // Held for the lifetime of the guard; dropping it releases the flock.
    _file: File,
    path: PathBuf,
}

impl InstanceGuard {
    pub fn acquire(path: &Path) -> Result<Self, InstanceError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                InstanceError::Io(format!(
                    "Instanz-Lock-Verzeichnis {} kann nicht erstellt werden: {err}",
                    parent.display()
                ))
            })?;
        }
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(|err| {
                InstanceError::Io(format!(
                    "Instanz-Lock {} kann nicht geöffnet werden: {err}",
                    path.display()
                ))
            })?;
        match file.try_lock_exclusive() {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                return Err(InstanceError::AlreadyRunning)
            }
            Err(err) => {
                return Err(InstanceError::Io(format!(
                    "Instanz-Lock {} kann nicht gesetzt werden: {err}",
                    path.display()
                )))
            }
        }
        let _ = file.set_len(0);
        let _ = file.seek(SeekFrom::Start(0));
        let _ = writeln!(file, "{}", std::process::id());
        let _ = file.sync_data();
        Ok(Self {
            _file: file,
            path: path.to_path_buf(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_instance_is_rejected_and_lock_recovers_after_drop() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("couchdeck.lock");
        let first = InstanceGuard::acquire(&path).unwrap();
        assert!(matches!(
            InstanceGuard::acquire(&path),
            Err(InstanceError::AlreadyRunning)
        ));
        drop(first);
        let second = InstanceGuard::acquire(&path).unwrap();
        assert_eq!(second.path(), path);
    }
}
