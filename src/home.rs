//! Optional user-level "CouchDeck Home" desktop launcher.
//!
//! The launcher is never installed automatically. `couchdeck
//! --install-home-launcher` is an explicit user action and writes only into
//! `$XDG_DATA_HOME/applications/`. The actual global key/controller binding
//! remains a user decision in the desktop environment/Steam Input.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::paths::XdgPaths;

pub const LAUNCHER_FILE: &str = "couchdeck-home.desktop";

pub fn launcher_path(paths: &XdgPaths) -> PathBuf {
    paths.data_home.join("applications").join(LAUNCHER_FILE)
}

/// Build a desktop entry for the installed binary or AppImage.
pub fn desktop_entry(executable: &Path) -> String {
    let executable = executable.to_string_lossy();
    // Desktop-entry Exec quoting: quote the path, escape backslashes/quotes,
    // and double literal percent signs so they are not interpreted as field
    // codes. The command itself is passed as separate arguments, never shell.
    let quoted = executable
        .replace('%', "%%")
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    format!(
        "[Desktop Entry]\n\
Type=Application\n\
Name=CouchDeck Home\n\
Name[de]=CouchDeck Home\n\
Comment=Return to CouchDeck or start it\n\
Comment[de]=Zu CouchDeck zurückkehren oder CouchDeck starten\n\
Exec=\"{quoted}\" --home\n\
Icon=couchdeck\n\
Terminal=false\n\
Categories=Game;Utility;\n\
StartupNotify=false\n\
X-CouchDeck-Managed=true\n"
    )
}

/// Write the desktop launcher atomically and return its path.
pub fn install_launcher(paths: &XdgPaths, executable: &Path) -> Result<PathBuf, String> {
    let path = launcher_path(paths);
    let parent = path
        .parent()
        .ok_or_else(|| "Ungültiger Home-Launcher-Pfad".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|err| format!("Launcher-Verzeichnis {}: {err}", parent.display()))?;
    let temp = parent.join(format!(".{LAUNCHER_FILE}.tmp-{}", std::process::id()));
    fs::write(&temp, desktop_entry(executable))
        .map_err(|err| format!("Home-Launcher {}: {err}", temp.display()))?;
    let _ = fs::set_permissions(&temp, fs::Permissions::from_mode(0o644));
    if let Err(err) = fs::rename(&temp, &path) {
        let _ = fs::remove_file(&temp);
        return Err(format!("Home-Launcher {}: {err}", path.display()));
    }
    Ok(path)
}

pub fn remove_launcher(paths: &XdgPaths) -> Result<bool, String> {
    let path = launcher_path(paths);
    match fs::remove_file(&path) {
        Ok(()) => Ok(true),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(format!("Home-Launcher {}: {err}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_entry_quotes_path_and_uses_home() {
        let entry = desktop_entry(Path::new("/tmp/Couch Deck/couchdeck"));
        assert!(entry.contains("Exec=\"/tmp/Couch Deck/couchdeck\" --home"));
        assert!(entry.contains("X-CouchDeck-Managed=true"));
        assert!(entry.contains("Icon=couchdeck"));
    }

    #[test]
    fn desktop_entry_escapes_desktop_specials() {
        let entry = desktop_entry(Path::new("/tmp/a%b\\c\"d"));
        assert!(entry.contains("a%%b\\\\c\\\"d"));
    }

    #[test]
    fn install_and_remove_launcher_are_atomic_and_explicit() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = XdgPaths::for_home(tmp.path());
        let exe = tmp.path().join("Couch Deck/couchdeck");
        let path = install_launcher(&paths, &exe).unwrap();
        assert_eq!(path, launcher_path(&paths));
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("--home"));
        assert!(!path.with_extension("tmp").exists());
        assert!(remove_launcher(&paths).unwrap());
        assert!(!remove_launcher(&paths).unwrap());
    }
}
