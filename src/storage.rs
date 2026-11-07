//! Durable, atomic persistence.
//!
//! Rules:
//! * never silently overwrite unreadable data – a timestamped backup is made
//!   before a corrupt file is replaced by an explicit user reset,
//! * library/settings writes are atomic (temp file + fsync + rename),
//! * PIN hashes are written with mode 0600.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::model::{
    AdminSecrets, Library, Settings, LIBRARY_SCHEMA_VERSION, SECRETS_SCHEMA_VERSION,
    SETTINGS_SCHEMA_VERSION,
};
use crate::paths::XdgPaths;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub enum LibraryLoad {
    Missing,
    Loaded(Library),
    /// The existing file was preserved and copied to `backup` where possible.
    /// The caller must ask the user before initialising a fresh library.
    Corrupt {
        message: String,
        backup: Option<PathBuf>,
    },
}

#[derive(Debug)]
pub enum SettingsLoad {
    Missing,
    Loaded(Settings),
    /// Falls back to defaults in memory. `backup` is a preserved copy.
    Corrupt {
        message: String,
        backup: Option<PathBuf>,
    },
}

#[derive(Debug)]
pub enum SecretsLoad {
    Missing,
    Loaded(AdminSecrets),
    Corrupt {
        message: String,
        backup: Option<PathBuf>,
    },
}

fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn unique_backup_path(path: &Path) -> PathBuf {
    let stamp = timestamp();
    let base = path.with_extension(format!(
        "{}.bak-{stamp}",
        path.extension().and_then(|e| e.to_str()).unwrap_or("file")
    ));
    if !base.exists() {
        return base;
    }
    for n in 2..1000 {
        let candidate = path.with_extension(format!(
            "{}.bak-{stamp}-{n}",
            path.extension().and_then(|e| e.to_str()).unwrap_or("file")
        ));
        if !candidate.exists() {
            return candidate;
        }
    }
    path.with_extension(format!("bak-{stamp}-{}", std::process::id()))
}

/// Copy an unreadable/corrupt file to a sibling backup. Never deletes or
/// renames the original. Returns the backup path on success.
pub fn backup_file(path: &Path) -> io::Result<PathBuf> {
    let backup = unique_backup_path(path);
    fs::copy(path, &backup)?;
    Ok(backup)
}

fn temp_path(parent: &Path) -> PathBuf {
    let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    parent.join(format!(
        ".couchdeck-{}-{}-{n}.tmp",
        std::process::id(),
        timestamp()
    ))
}

/// Atomically write bytes to `path` in the same directory.
pub fn atomic_write(path: &Path, bytes: &[u8], private: bool) -> io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Pfad ohne Elternverzeichnis: {}", path.display()),
        )
    })?;
    fs::create_dir_all(parent)?;
    let tmp = temp_path(parent);
    let result = (|| -> io::Result<()> {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            if private {
                options.mode(0o600);
            } else {
                options.mode(0o644);
            }
        }
        let mut file = options.open(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&tmp, path)?;
        // Make sure the rename itself is durable on real filesystems.
        if let Ok(dir) = File::open(parent) {
            let _ = dir.sync_all();
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

pub fn load_library(paths: &XdgPaths) -> LibraryLoad {
    let path = paths.library_path();
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return LibraryLoad::Missing,
        Err(err) => {
            return LibraryLoad::Corrupt {
                message: format!("{} konnte nicht gelesen werden: {err}", path.display()),
                backup: None,
            }
        }
    };
    let mut library: Library = match serde_json::from_slice(&bytes) {
        Ok(lib) => lib,
        Err(err) => {
            let backup = backup_file(&path).ok();
            return LibraryLoad::Corrupt {
                message: format!(
                    "library.json ist kein gültiges JSON ({err}). Die Datei wurde nicht überschrieben."
                ),
                backup,
            };
        }
    };
    if library.schema_version > LIBRARY_SCHEMA_VERSION {
        let backup = backup_file(&path).ok();
        return LibraryLoad::Corrupt {
            message: format!(
                "library.json verwendet Schema {}, CouchDeck unterstützt höchstens Schema {}. Bitte eine neuere CouchDeck-Version verwenden.",
                library.schema_version, LIBRARY_SCHEMA_VERSION
            ),
            backup,
        };
    }
    if library.schema_version < LIBRARY_SCHEMA_VERSION {
        // No migration exists yet. Preserve the old file before upgrading the
        // in-memory schema version.
        let _ = backup_file(&path);
        library.schema_version = LIBRARY_SCHEMA_VERSION;
    }
    library.normalize();
    LibraryLoad::Loaded(library)
}

pub fn save_library(paths: &XdgPaths, library: &Library) -> Result<(), String> {
    let path = paths.library_path();
    let json = serde_json::to_vec_pretty(library)
        .map_err(|err| format!("Bibliothek konnte nicht serialisiert werden: {err}"))?;
    atomic_write(&path, &json, false).map_err(|err| {
        format!(
            "Bibliothek konnte nicht gespeichert werden ({}): {err}",
            path.display()
        )
    })
}

pub fn load_settings(paths: &XdgPaths) -> SettingsLoad {
    let path = paths.settings_path();
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return SettingsLoad::Missing,
        Err(err) => {
            return SettingsLoad::Corrupt {
                message: format!("Einstellungen konnten nicht gelesen werden: {err}"),
                backup: None,
            }
        }
    };
    match toml::from_str::<Settings>(&text) {
        Ok(mut settings) => {
            if settings.schema_version > SETTINGS_SCHEMA_VERSION {
                let backup = backup_file(&path).ok();
                return SettingsLoad::Corrupt {
                    message: format!(
                        "settings.toml verwendet Schema {} statt höchstens {}.",
                        settings.schema_version, SETTINGS_SCHEMA_VERSION
                    ),
                    backup,
                };
            }
            settings.schema_version = SETTINGS_SCHEMA_VERSION;
            SettingsLoad::Loaded(settings)
        }
        Err(err) => {
            let backup = backup_file(&path).ok();
            SettingsLoad::Corrupt {
                message: format!(
                    "settings.toml ist ungültig ({err}). Es werden Standardwerte verwendet; die Datei bleibt erhalten."
                ),
                backup,
            }
        }
    }
}

pub fn save_settings(paths: &XdgPaths, settings: &Settings) -> Result<(), String> {
    let path = paths.settings_path();
    let text = toml::to_string_pretty(settings)
        .map_err(|err| format!("Einstellungen konnten nicht serialisiert werden: {err}"))?;
    atomic_write(&path, text.as_bytes(), false)
        .map_err(|err| format!("Einstellungen konnten nicht gespeichert werden: {err}"))
}

pub fn load_secrets(paths: &XdgPaths) -> SecretsLoad {
    let path = paths.secrets_path();
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return SecretsLoad::Missing,
        Err(err) => {
            return SecretsLoad::Corrupt {
                message: format!("PIN-Datei konnte nicht gelesen werden: {err}"),
                backup: None,
            }
        }
    };
    match serde_json::from_slice::<AdminSecrets>(&bytes) {
        Ok(mut secrets) => {
            if secrets.schema_version > SECRETS_SCHEMA_VERSION {
                let backup = backup_file(&path).ok();
                return SecretsLoad::Corrupt {
                    message: "PIN-Datei stammt von einer neueren CouchDeck-Version.".into(),
                    backup,
                };
            }
            secrets.schema_version = SECRETS_SCHEMA_VERSION;
            SecretsLoad::Loaded(secrets)
        }
        Err(err) => {
            let backup = backup_file(&path).ok();
            SecretsLoad::Corrupt {
                message: format!(
                    "PIN-Datei ist beschädigt ({err}). Sie wurde gesichert und wird erst beim nächsten Setzen einer PIN neu geschrieben."
                ),
                backup,
            }
        }
    }
}

pub fn save_secrets(paths: &XdgPaths, secrets: &AdminSecrets) -> Result<(), String> {
    let path = paths.secrets_path();
    let json = serde_json::to_vec_pretty(secrets)
        .map_err(|err| format!("PIN-Datei konnte nicht serialisiert werden: {err}"))?;
    atomic_write(&path, &json, true)
        .map_err(|err| format!("PIN-Datei konnte nicht gespeichert werden: {err}"))
}

/// Explicit user reset after a corrupt-library dialog. The original is backed
/// up first and the fresh library is written atomically.
pub fn reset_library_with_backup(paths: &XdgPaths) -> Result<(Library, Option<PathBuf>), String> {
    let path = paths.library_path();
    let backup = if path.exists() {
        Some(backup_file(&path).map_err(|err| format!("Backup fehlgeschlagen: {err}"))?)
    } else {
        None
    };
    let library = Library::new();
    save_library(paths, &library)?;
    Ok((library, backup))
}

pub fn ensure_directories(paths: &XdgPaths) -> Result<(), String> {
    for dir in [
        paths.config_dir(),
        paths.data_dir(),
        paths.cache_dir(),
        paths.state_dir(),
        paths.assets_dir(),
        paths.covers_dir(),
        paths.backgrounds_dir(),
        paths.avatars_dir(),
        paths.managed_desktop_dir(),
        paths.thumbnails_dir(),
    ] {
        fs::create_dir_all(&dir).map_err(|err| {
            format!(
                "Verzeichnis {} kann nicht erstellt werden: {err}",
                dir.display()
            )
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn test_paths() -> (tempfile::TempDir, XdgPaths) {
        let tmp = tempfile::tempdir().unwrap();
        let paths = XdgPaths::for_home(tmp.path());
        ensure_directories(&paths).unwrap();
        (tmp, paths)
    }

    #[test]
    fn atomic_write_replaces_and_keeps_no_temp() {
        let (_tmp, paths) = test_paths();
        let file = paths.data_dir().join("x.txt");
        atomic_write(&file, b"first", false).unwrap();
        atomic_write(&file, b"second", false).unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "second");
        let leftover_tmp = fs::read_dir(file.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .any(|e| e.file_name().to_string_lossy().ends_with(".tmp"));
        assert!(!leftover_tmp);
    }

    #[test]
    fn library_roundtrip() {
        let (_tmp, paths) = test_paths();
        let mut library = Library::new();
        library.games.push(crate::model::GameEntry {
            id: "g".into(),
            desktop_file: PathBuf::from("/tmp/game.desktop"),
            title_override: None,
            desktop_name: Some("Game".into()),
            desktop_icon: None,
            cover: None,
            background: None,
            visible_to_profiles: None,
            hidden: false,
            sort_order: 10,
            managed: false,
            created_at_unix: 0,
            desktop_id: None,
            source: Default::default(),
            launch_method: Default::default(),
        });
        save_library(&paths, &library).unwrap();
        match load_library(&paths) {
            LibraryLoad::Loaded(lib) => {
                assert_eq!(lib.games.len(), 1);
                assert_eq!(lib.games[0].title(), "Game");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn corrupt_library_is_preserved() {
        let (_tmp, paths) = test_paths();
        let path = paths.library_path();
        fs::write(&path, b"{ this is not json").unwrap();
        match load_library(&paths) {
            LibraryLoad::Corrupt { backup, .. } => {
                assert!(backup.is_some());
                assert!(backup.unwrap().exists());
                assert_eq!(fs::read(&path).unwrap(), b"{ this is not json");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn newer_schema_is_not_accepted() {
        let (_tmp, paths) = test_paths();
        let path = paths.library_path();
        fs::write(&path, br#"{"schema_version":99,"profiles":[],"games":[]}"#).unwrap();
        match load_library(&paths) {
            LibraryLoad::Corrupt { message, .. } => assert!(message.contains("Schema")),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn secrets_are_private_and_roundtrip() {
        let (_tmp, paths) = test_paths();
        let mut secrets = AdminSecrets::new();
        secrets.set_pin("admin", "1234").unwrap();
        save_secrets(&paths, &secrets).unwrap();
        let mode = fs::metadata(paths.secrets_path())
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
        match load_secrets(&paths) {
            SecretsLoad::Loaded(loaded) => assert!(loaded.verify("admin", "1234")),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn corrupt_settings_use_defaults_and_are_backed_up() {
        let (_tmp, paths) = test_paths();
        let path = paths.settings_path();
        fs::write(&path, "this is [not toml").unwrap();
        match load_settings(&paths) {
            SettingsLoad::Corrupt { backup, .. } => {
                assert!(backup.as_ref().is_some_and(|b| b.exists()));
                assert_eq!(fs::read_to_string(&path).unwrap(), "this is [not toml");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn corrupt_secrets_are_backed_up_and_do_not_overwrite() {
        let (_tmp, paths) = test_paths();
        let path = paths.secrets_path();
        fs::write(&path, "{ bad secrets").unwrap();
        match load_secrets(&paths) {
            SecretsLoad::Corrupt { backup, .. } => {
                assert!(backup.as_ref().is_some_and(|b| b.exists()));
                assert_eq!(fs::read_to_string(&path).unwrap(), "{ bad secrets");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn settings_roundtrip() {
        let (_tmp, paths) = test_paths();
        let settings = Settings {
            animations: false,
            additional_desktop_dirs: vec![PathBuf::from("/tmp/extra games")],
            ..Default::default()
        };
        save_settings(&paths, &settings).unwrap();
        match load_settings(&paths) {
            SettingsLoad::Loaded(loaded) => {
                assert!(!loaded.animations);
                assert_eq!(
                    loaded.additional_desktop_dirs,
                    vec![PathBuf::from("/tmp/extra games")]
                );
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn reset_creates_backup() {
        let (_tmp, paths) = test_paths();
        fs::write(paths.library_path(), b"broken").unwrap();
        let (_lib, backup) = reset_library_with_backup(&paths).unwrap();
        assert!(backup.unwrap().exists());
        assert!(matches!(load_library(&paths), LibraryLoad::Loaded(_)));
    }
}
