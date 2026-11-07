//! Desktop-session detection and desktop-independent XDG autostart.
//!
//! CouchDeck targets ordinary Linux desktop sessions. Autostart is opt-in and
//! restricted to the desktop environment that is actually active. Unknown or
//! gamescope/Steam-Gaming-Mode sessions are never guessed.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::paths::XdgPaths;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopEnvironment {
    Kde,
    Gnome,
    Xfce,
}

impl DesktopEnvironment {
    pub fn token(self) -> &'static str {
        match self {
            Self::Kde => "KDE",
            Self::Gnome => "GNOME",
            Self::Xfce => "XFCE",
        }
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Self::Kde => "desktop-kde",
            Self::Gnome => "desktop-gnome",
            Self::Xfce => "desktop-xfce",
        }
    }

    pub fn from_token(token: &str) -> Option<Self> {
        let token = token.trim().to_ascii_uppercase();
        if token == "KDE" || token.contains("PLASMA") {
            Some(Self::Kde)
        } else if token == "GNOME" || token.starts_with("GNOME") {
            Some(Self::Gnome)
        } else if token == "XFCE" || token.starts_with("XFCE") {
            Some(Self::Xfce)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionInfo {
    pub environment: Option<DesktopEnvironment>,
    pub gaming_mode: bool,
    /// Normalized tokens from the session environment, in detection order.
    pub tokens: Vec<String>,
}

impl SessionInfo {
    pub fn is_supported_desktop(&self) -> bool {
        self.environment.is_some() && !self.gaming_mode
    }
}

fn split_tokens(value: &str) -> impl Iterator<Item = String> + '_ {
    value
        .split([':', ';'])
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(str::to_ascii_uppercase)
}

/// Testable session detection from the three relevant environment values.
pub fn detect_from_values(current: &str, session: &str, desktop: &str) -> SessionInfo {
    let mut tokens = Vec::new();
    for value in [current, session, desktop] {
        for token in split_tokens(value) {
            if !tokens.contains(&token) {
                tokens.push(token);
            }
        }
    }
    let gaming_mode = tokens
        .iter()
        .any(|token| token.contains("GAMESCOPE") || token.contains("GAMINGMODE"));
    let environment = if gaming_mode {
        None
    } else {
        tokens
            .iter()
            .find_map(|token| DesktopEnvironment::from_token(token))
    };
    SessionInfo {
        environment,
        gaming_mode,
        tokens,
    }
}

pub fn detect_current() -> SessionInfo {
    detect_from_values(
        &env::var("XDG_CURRENT_DESKTOP").unwrap_or_default(),
        &env::var("XDG_SESSION_DESKTOP").unwrap_or_default(),
        &env::var("DESKTOP_SESSION").unwrap_or_default(),
    )
}

pub fn current_desktop_tokens() -> Vec<String> {
    let session = detect_current();
    let mut tokens = session.tokens.clone();
    if let Some(environment) = session.environment {
        if !tokens.iter().any(|token| token == environment.token()) {
            tokens.push(environment.token().to_owned());
        }
    }
    tokens
}

pub fn autostart_supported_session() -> Option<DesktopEnvironment> {
    let session = detect_current();
    session.is_supported_desktop().then_some(
        session
            .environment
            .expect("is_supported_desktop requires an environment"),
    )
}

pub fn autostart_file_exists(paths: &XdgPaths) -> bool {
    paths.autostart_path().is_file()
}

/// True only when the existing autostart entry applies to the current,
/// supported desktop session. A KDE entry is therefore inactive on GNOME/XFCE
/// and gamescope sessions are never considered active.
pub fn autostart_enabled(paths: &XdgPaths) -> bool {
    let Some(environment) = autostart_supported_session() else {
        return false;
    };
    let Ok(text) = fs::read_to_string(paths.autostart_path()) else {
        return false;
    };
    entry_applies_to_environment(&text, environment)
}

pub fn entry_applies_to_environment(contents: &str, environment: DesktopEnvironment) -> bool {
    entry_only_show_in(contents)
        .iter()
        .any(|token| token.eq_ignore_ascii_case(environment.token()))
}

pub fn entry_only_show_in(contents: &str) -> Vec<String> {
    for line in contents.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("OnlyShowIn=") {
            return value
                .split(';')
                .map(str::trim)
                .filter(|token| !token.is_empty())
                .map(str::to_owned)
                .collect();
        }
    }
    Vec::new()
}

pub fn write_autostart(paths: &XdgPaths, binary: &Path) -> Result<PathBuf, String> {
    let environment = autostart_supported_session().ok_or_else(|| {
        let session = detect_current();
        if session.gaming_mode {
            "Autostart ist in Steam Gaming Mode/Gamescope nicht aktiviert. Bitte im Desktop-Modus konfigurieren.".to_owned()
        } else {
            "Autostart wird nur für KDE Plasma, GNOME und XFCE angeboten; die aktuelle Desktop-Sitzung wurde nicht erkannt.".to_owned()
        }
    })?;
    write_autostart_for(paths, binary, environment)
}

pub fn write_autostart_for(
    paths: &XdgPaths,
    binary: &Path,
    environment: DesktopEnvironment,
) -> Result<PathBuf, String> {
    let binary = binary
        .to_str()
        .ok_or_else(|| "Der Programmpfad enthält kein gültiges UTF-8.".to_owned())?;
    let contents = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=CouchDeck\n\
         Comment=Family-friendly, controller-first game launcher\n\
         Exec={} --autostart\n\
         Icon=couchdeck\n\
         Terminal=false\n\
         Categories=Game;\n\
         StartupNotify=false\n\
         OnlyShowIn={};\n\
         NotShowIn=gamescope;\n\
         X-GNOME-Autostart-enabled=true\n\
         X-CouchDeck-Autostart=true\n\
         X-CouchDeck-Desktop={};\n",
        crate::desktop::exec_quote(binary),
        environment.token(),
        environment.token()
    );
    let path = paths.autostart_path();
    crate::storage::atomic_write(&path, contents.as_bytes(), false)
        .map_err(|err| format!("Autostart-Datei konnte nicht geschrieben werden: {err}"))?;
    Ok(path)
}

pub fn remove_autostart(paths: &XdgPaths) -> Result<bool, String> {
    let path = paths.autostart_path();
    if !path.exists() {
        return Ok(false);
    }
    fs::remove_file(&path)
        .map_err(|err| format!("Autostart-Datei konnte nicht entfernt werden: {err}"))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_multi_token_desktops() {
        let gnome = detect_from_values("ubuntu:GNOME", "", "");
        assert_eq!(gnome.environment, Some(DesktopEnvironment::Gnome));
        assert!(gnome.is_supported_desktop());
        assert_eq!(
            detect_from_values("KDE", "", "").environment,
            Some(DesktopEnvironment::Kde)
        );
        assert_eq!(
            detect_from_values("XFCE", "", "").environment,
            Some(DesktopEnvironment::Xfce)
        );
        assert_eq!(detect_from_values("", "", "").environment, None);
        assert_eq!(detect_from_values("sway", "", "").environment, None);
    }

    #[test]
    fn gamescope_is_never_treated_as_desktop_session() {
        let session = detect_from_values("gamescope", "gamescope", "");
        assert!(session.gaming_mode);
        assert_eq!(session.environment, None);
        assert!(!session.is_supported_desktop());
        let mixed = detect_from_values("gamescope:KDE", "", "");
        assert!(mixed.gaming_mode);
        assert_eq!(mixed.environment, None);
    }

    #[test]
    fn autostart_is_bound_to_current_desktop_and_not_kde_only() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = XdgPaths::for_home(tmp.path());
        let binary = tmp.path().join("My Games/couchdeck");
        let path = write_autostart_for(&paths, &binary, DesktopEnvironment::Gnome).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("OnlyShowIn=GNOME;"));
        assert!(text.contains("NotShowIn=gamescope;"));
        assert!(text.contains("X-CouchDeck-Desktop=GNOME;"));
        assert!(!text.contains("OnlyShowIn=KDE;"));
        assert!(!text.contains("X-KDE-autostart-after"));
        assert!(text.contains("--autostart"));
        assert_eq!(entry_only_show_in(&text), vec!["GNOME"]);
        assert!(entry_applies_to_environment(
            &text,
            DesktopEnvironment::Gnome
        ));
        assert!(!entry_applies_to_environment(
            &text,
            DesktopEnvironment::Kde
        ));
        assert!(remove_autostart(&paths).unwrap());
        assert!(!autostart_file_exists(&paths));
        assert!(!remove_autostart(&paths).unwrap());
    }
}
