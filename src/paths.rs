//! XDG base directory handling.
//!
//! No hard-coded user names. Environment variables are honoured, with the
//! standard `$HOME/...` fallbacks required by the XDG base directory spec.

use std::collections::BTreeSet;
use std::env;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathError(pub String);

impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for PathError {}

#[derive(Debug, Clone)]
pub struct XdgPaths {
    pub home: PathBuf,
    pub config_home: PathBuf,
    pub data_home: PathBuf,
    pub cache_home: PathBuf,
    pub state_home: PathBuf,
    pub data_dirs: Vec<PathBuf>,
    flatpak_application_dirs: Vec<PathBuf>,
}

impl XdgPaths {
    /// Resolve paths from the real process environment.
    pub fn discover() -> Result<Self, PathError> {
        let home = env::var_os("HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .ok_or_else(|| {
                PathError("HOME ist nicht gesetzt; XDG-Pfade können nicht ermittelt werden.".into())
            })?;
        Self::from_env(&home, |key| env::var_os(key))
    }

    /// Testable constructor. `get_env` can be replaced by a closure in tests.
    pub fn from_env(
        home: &Path,
        get_env: impl Fn(&str) -> Option<OsString>,
    ) -> Result<Self, PathError> {
        Self::from_env_impl(home, get_env, true)
    }

    /// Like [`Self::from_env`], but does not probe the real filesystem for
    /// Flatpak export directories. Used by tests that need an isolated XDG
    /// environment.
    pub fn from_env_isolated(
        home: &Path,
        get_env: impl Fn(&str) -> Option<OsString>,
    ) -> Result<Self, PathError> {
        Self::from_env_impl(home, get_env, false)
    }

    fn from_env_impl(
        home: &Path,
        get_env: impl Fn(&str) -> Option<OsString>,
        include_flatpak_exports: bool,
    ) -> Result<Self, PathError> {
        let non_empty = |key: &str| get_env(key).filter(|v| !v.is_empty()).map(PathBuf::from);
        let config_home = non_empty("XDG_CONFIG_HOME").unwrap_or_else(|| home.join(".config"));
        let data_home = non_empty("XDG_DATA_HOME").unwrap_or_else(|| home.join(".local/share"));
        let cache_home = non_empty("XDG_CACHE_HOME").unwrap_or_else(|| home.join(".cache"));
        let state_home = non_empty("XDG_STATE_HOME").unwrap_or_else(|| home.join(".local/state"));

        let raw_data_dirs = get_env("XDG_DATA_DIRS")
            .unwrap_or_else(|| OsString::from("/usr/local/share:/usr/share"));

        let mut data_dirs = Vec::new();
        let mut seen = BTreeSet::new();
        for part in raw_data_dirs.to_string_lossy().split(':') {
            if part.is_empty() {
                continue;
            }
            let path = PathBuf::from(part);
            if seen.insert(path.clone()) {
                data_dirs.push(path);
            }
        }
        // Flatpak exports are normally part of XDG_DATA_DIRS on desktop systems, but
        // make sure icon/desktop discovery also works when CouchDeck is started
        // from a minimal environment.
        if include_flatpak_exports {
            for extra in [
                PathBuf::from("/var/lib/flatpak/exports/share"),
                data_home.join("flatpak/exports/share"),
            ] {
                if extra.is_dir() && seen.insert(extra.clone()) {
                    data_dirs.insert(0, extra);
                }
            }
        }

        let flatpak_application_dirs = if include_flatpak_exports {
            vec![
                data_home.join("flatpak/exports/share/applications"),
                PathBuf::from("/var/lib/flatpak/exports/share/applications"),
            ]
        } else {
            vec![data_home.join("flatpak/exports/share/applications")]
        };

        Ok(Self {
            home: home.to_path_buf(),
            config_home,
            data_home,
            cache_home,
            state_home,
            data_dirs,
            flatpak_application_dirs,
        })
    }

    /// Convenience constructor for tests and for callers that do not care about
    /// the process environment's data dirs.
    pub fn for_home(home: impl Into<PathBuf>) -> Self {
        Self::from_env(&home.into(), |_| None).expect("home path is always valid")
    }

    pub fn config_dir(&self) -> PathBuf {
        self.config_home.join("couchdeck")
    }

    pub fn data_dir(&self) -> PathBuf {
        self.data_home.join("couchdeck")
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.cache_home.join("couchdeck")
    }

    pub fn state_dir(&self) -> PathBuf {
        self.state_home.join("couchdeck")
    }

    pub fn settings_path(&self) -> PathBuf {
        self.config_dir().join("settings.toml")
    }

    pub fn library_path(&self) -> PathBuf {
        self.data_dir().join("library.json")
    }

    pub fn secrets_path(&self) -> PathBuf {
        self.data_dir().join("secrets.json")
    }

    pub fn timekeeping_path(&self) -> PathBuf {
        self.data_dir().join("timekeeping.json")
    }

    pub fn assets_dir(&self) -> PathBuf {
        self.data_dir().join("assets")
    }

    pub fn covers_dir(&self) -> PathBuf {
        self.assets_dir().join("covers")
    }

    pub fn backgrounds_dir(&self) -> PathBuf {
        self.assets_dir().join("backgrounds")
    }

    pub fn avatars_dir(&self) -> PathBuf {
        self.assets_dir().join("avatars")
    }

    pub fn managed_desktop_dir(&self) -> PathBuf {
        self.data_dir().join("managed-desktop")
    }

    pub fn thumbnails_dir(&self) -> PathBuf {
        self.cache_dir().join("thumbnails")
    }

    pub fn log_path(&self) -> PathBuf {
        self.state_dir().join("couchdeck.log")
    }

    /// XDG autostart entry. Its `OnlyShowIn=` value is generated from the
    /// active desktop session (KDE, GNOME or XFCE).
    pub fn autostart_path(&self) -> PathBuf {
        self.config_home.join("autostart/couchdeck.desktop")
    }

    /// Instance lock. Prefer the per-user runtime directory; fall back to the
    /// state directory when XDG_RUNTIME_DIR is unavailable.
    pub fn instance_lock_path(&self) -> PathBuf {
        if let Some(runtime) = env::var_os("XDG_RUNTIME_DIR").filter(|value| !value.is_empty()) {
            PathBuf::from(runtime).join("couchdeck.lock")
        } else {
            self.state_dir().join("instance.lock")
        }
    }

    /// The real XDG desktop directory from `user-dirs.dirs`, not a hard-coded
    /// `~/Desktop` assumption. Resolves the `$HOME` token as defined by the
    /// XDG user-dirs specification.
    pub fn desktop_dir(&self) -> Option<PathBuf> {
        let user_dirs = self.config_home.join("user-dirs.dirs");
        if let Ok(text) = fs::read_to_string(&user_dirs) {
            for line in text.lines() {
                let line = line.trim();
                let Some(raw) = line.strip_prefix("XDG_DESKTOP_DIR=") else {
                    continue;
                };
                let raw = raw.trim().trim_matches('"');
                let home = self.home.to_string_lossy();
                let expanded = raw.replace("${HOME}", &home).replace("$HOME", &home);
                let path = PathBuf::from(expanded);
                let path = if path.is_absolute() {
                    path
                } else {
                    self.home.join(path)
                };
                return path.is_dir().then_some(path);
            }
        }
        let fallback = self.home.join("Desktop");
        fallback.is_dir().then_some(fallback)
    }

    /// Standard Flatpak desktop exports. XDG_DATA_DIRS may already contain
    /// these; callers de-duplicate by normalized path.
    pub fn flatpak_application_dirs(&self) -> Vec<PathBuf> {
        self.flatpak_application_dirs.clone()
    }

    /// Application directories in XDG precedence order (user first).
    pub fn application_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        let mut seen = BTreeSet::new();
        let mut add = |p: PathBuf| {
            if seen.insert(p.clone()) {
                dirs.push(p);
            }
        };
        add(self.data_home.join("applications"));
        for base in &self.data_dirs {
            add(base.join("applications"));
        }
        dirs
    }

    pub fn icon_search_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        let mut seen = BTreeSet::new();
        let mut add = |p: PathBuf| {
            if seen.insert(p.clone()) {
                dirs.push(p);
            }
        };
        add(self.data_home.join("icons"));
        add(self.data_home.join("pixmaps"));
        for base in &self.data_dirs {
            add(base.join("icons"));
            add(base.join("pixmaps"));
        }
        add(self.home.join(".icons"));
        add(PathBuf::from("/usr/share/pixmaps"));
        dirs
    }

    pub fn data_relative(&self, absolute: &Path) -> Option<PathBuf> {
        absolute
            .strip_prefix(self.data_dir())
            .ok()
            .map(Path::to_path_buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn honours_xdg_variables() {
        let paths = XdgPaths::from_env_isolated(Path::new("/home/tester"), |key| match key {
            "XDG_CONFIG_HOME" => Some(OsString::from("/tmp/cfg")),
            "XDG_DATA_HOME" => Some(OsString::from("/tmp/data")),
            "XDG_CACHE_HOME" => Some(OsString::from("/tmp/cache")),
            "XDG_STATE_HOME" => Some(OsString::from("/tmp/state")),
            "XDG_DATA_DIRS" => Some(OsString::from("/one:/two:/one")),
            _ => None,
        })
        .unwrap();
        assert_eq!(
            paths.settings_path(),
            Path::new("/tmp/cfg/couchdeck/settings.toml")
        );
        assert_eq!(
            paths.library_path(),
            Path::new("/tmp/data/couchdeck/library.json")
        );
        assert_eq!(
            paths.secrets_path(),
            Path::new("/tmp/data/couchdeck/secrets.json")
        );
        assert_eq!(paths.cache_dir(), Path::new("/tmp/cache/couchdeck"));
        assert_eq!(paths.state_dir(), Path::new("/tmp/state/couchdeck"));
        assert_eq!(
            paths.data_dirs,
            vec![PathBuf::from("/one"), PathBuf::from("/two")]
        );
        let apps = paths.application_dirs();
        assert_eq!(apps[0], Path::new("/tmp/data/applications"));
        assert!(apps.contains(&PathBuf::from("/one/applications")));
    }

    #[test]
    fn resolves_actual_xdg_desktop_dir() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join(".config")).unwrap();
        fs::create_dir_all(tmp.path().join("Mein Schreibtisch")).unwrap();
        fs::write(
            tmp.path().join(".config/user-dirs.dirs"),
            "XDG_DESKTOP_DIR=\"$HOME/Mein Schreibtisch\"\n",
        )
        .unwrap();
        let paths = XdgPaths::for_home(tmp.path());
        assert_eq!(
            paths.desktop_dir(),
            Some(tmp.path().join("Mein Schreibtisch"))
        );
    }

    #[test]
    fn falls_back_to_home() {
        let paths = XdgPaths::from_env(Path::new("/home/x"), |_| None).unwrap();
        assert_eq!(paths.config_dir(), Path::new("/home/x/.config/couchdeck"));
        assert_eq!(
            paths.data_dir(),
            Path::new("/home/x/.local/share/couchdeck")
        );
        assert_eq!(
            paths.thumbnails_dir(),
            Path::new("/home/x/.cache/couchdeck/thumbnails")
        );
    }
}
