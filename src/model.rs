//! Persistent data model for CouchDeck.
//!
//! The on-disk contract is versioned. `library.json` contains profiles and
//! game entries, `settings.toml` contains UI preferences and the admin PIN
//! hashes live in a separate, private `secrets.json`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::i18n::LanguagePreference;

pub const LIBRARY_SCHEMA_VERSION: u32 = 1;
pub const SETTINGS_SCHEMA_VERSION: u32 = 1;
pub const SECRETS_SCHEMA_VERSION: u32 = 1;

fn schema_v1() -> u32 {
    LIBRARY_SCHEMA_VERSION
}

fn settings_v1() -> u32 {
    SETTINGS_SCHEMA_VERSION
}

fn secrets_v1() -> u32 {
    SECRETS_SCHEMA_VERSION
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileRole {
    Admin,
    Player,
}

impl ProfileRole {
    pub fn is_admin(self) -> bool {
        matches!(self, Self::Admin)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Admin => "Admin",
            Self::Player => "Spieler",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    #[serde(default = "default_role")]
    pub role: ProfileRole,
    /// Path to an imported avatar, relative to the CouchDeck data directory.
    #[serde(default)]
    pub avatar: Option<PathBuf>,
    /// Optional per-profile favourites (game ids). Ordering in the grid is
    /// still governed by `GameEntry::sort_order`; this is reserved for a later
    /// UI refinement but persisted now so no data is lost.
    #[serde(default)]
    pub favorites: Vec<String>,
    /// Optional profile-specific background, relative to the CouchDeck data
    /// directory. `None` falls back to the global background.
    #[serde(default)]
    pub background: Option<PathBuf>,
    /// Whether parental time budgeting applies to this profile.
    #[serde(default)]
    pub is_child: bool,
    /// Optional daily budget in minutes. `None` means no time limit.
    #[serde(default)]
    pub daily_limit_minutes: Option<u32>,
}

fn default_role() -> ProfileRole {
    ProfileRole::Player
}

impl Profile {
    pub fn new_admin(name: impl Into<String>) -> Self {
        Self {
            id: "admin".to_owned(),
            name: name.into(),
            role: ProfileRole::Admin,
            avatar: None,
            favorites: Vec::new(),
            background: None,
            is_child: false,
            daily_limit_minutes: None,
        }
    }

    pub fn is_admin(&self) -> bool {
        self.role.is_admin()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GameSource {
    #[default]
    DesktopEntry,
    ManagedDesktop,
    Steam,
    Prism,
    Manual,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LaunchMethod {
    #[default]
    GioDesktop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEntry {
    pub id: String,
    /// Absolute path to the original or managed `.desktop` file. The source
    /// file is never modified by CouchDeck.
    pub desktop_file: PathBuf,
    #[serde(default)]
    pub title_override: Option<String>,
    /// Last known `Name=` value, used when the desktop file disappears.
    #[serde(default)]
    pub desktop_name: Option<String>,
    /// Last known `Icon=` value, used when the desktop file disappears.
    #[serde(default)]
    pub desktop_icon: Option<String>,
    /// Cover override, relative to the CouchDeck data directory.
    #[serde(default)]
    pub cover: Option<PathBuf>,
    /// Background override, relative to the CouchDeck data directory.
    #[serde(default)]
    pub background: Option<PathBuf>,
    /// `None` means visible to every profile. `Some(vec)` restricts visibility.
    #[serde(default)]
    pub visible_to_profiles: Option<Vec<String>>,
    /// Administrator-hidden entry. The desktop file itself remains untouched.
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub sort_order: i64,
    /// True when CouchDeck generated this desktop file in `managed-desktop/`.
    #[serde(default)]
    pub managed: bool,
    #[serde(default)]
    pub created_at_unix: u64,
    /// XDG desktop-file id when the entry was imported from a discovery root.
    #[serde(default)]
    pub desktop_id: Option<String>,
    /// Origin of the entry. `desktop_file` stays the launch source.
    #[serde(default)]
    pub source: GameSource,
    /// How CouchDeck launches the entry. Only GIO desktop launches exist in 0.4.
    #[serde(default)]
    pub launch_method: LaunchMethod,
}

impl GameEntry {
    pub fn title(&self) -> String {
        if let Some(title) = self
            .title_override
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            return title.to_owned();
        }
        if let Some(name) = self
            .desktop_name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            return name.to_owned();
        }
        self.desktop_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unbekannter Eintrag")
            .to_owned()
    }

    pub fn is_visible_to(&self, profile_id: &str) -> bool {
        if self.hidden {
            return false;
        }
        match &self.visible_to_profiles {
            None => true,
            Some(ids) => ids.iter().any(|id| id == profile_id),
        }
    }

    pub fn desktop_exists(&self) -> bool {
        self.desktop_file.is_file()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    #[serde(default = "schema_v1")]
    pub schema_version: u32,
    #[serde(default)]
    pub active_profile_id: Option<String>,
    #[serde(default)]
    pub profiles: Vec<Profile>,
    #[serde(default)]
    pub games: Vec<GameEntry>,
}

impl Default for Library {
    fn default() -> Self {
        Self::new()
    }
}

impl Library {
    pub fn new() -> Self {
        let admin = Profile::new_admin("Spieler 1");
        Self {
            schema_version: LIBRARY_SCHEMA_VERSION,
            active_profile_id: Some(admin.id.clone()),
            profiles: vec![admin],
            games: Vec::new(),
        }
    }

    /// Repair invariants after loading data that may have been edited by hand.
    /// This never silently discards games or profiles.
    pub fn normalize(&mut self) {
        if self.profiles.is_empty() {
            self.profiles.push(Profile::new_admin("Spieler 1"));
        }
        if !self.profiles.iter().any(|p| p.is_admin()) {
            if let Some(first) = self.profiles.first_mut() {
                first.role = ProfileRole::Admin;
            }
        }
        // Make IDs unique without dropping data: rename duplicates with a suffix.
        let mut seen = std::collections::HashSet::new();
        for profile in &mut self.profiles {
            let base = profile.id.clone();
            let mut candidate = base.clone();
            let mut n = 2;
            while !seen.insert(candidate.clone()) {
                candidate = format!("{base}-{n}");
                n += 1;
            }
            profile.id = candidate;
            if profile.name.trim().is_empty() {
                profile.name = "Profil".to_owned();
            }
        }
        if self
            .active_profile_id
            .as_ref()
            .map(|id| !self.profiles.iter().any(|p| &p.id == id))
            .unwrap_or(true)
        {
            self.active_profile_id = self
                .profiles
                .iter()
                .find(|p| p.is_admin())
                .or_else(|| self.profiles.first())
                .map(|p| p.id.clone());
        }

        let mut seen_games = std::collections::HashSet::new();
        for (index, game) in self.games.iter_mut().enumerate() {
            let base = game.id.clone();
            let mut candidate = base.clone();
            let mut n = 2;
            while !seen_games.insert(candidate.clone()) {
                candidate = format!("{base}-{n}");
                n += 1;
            }
            game.id = candidate;
            if game.sort_order == 0 {
                game.sort_order = (index as i64 + 1) * 10;
            }
            if let Some(ids) = &mut game.visible_to_profiles {
                ids.retain(|id| self.profiles.iter().any(|p| &p.id == id));
            }
        }
    }

    pub fn active_profile(&self) -> Option<&Profile> {
        let id = self.active_profile_id.as_deref()?;
        self.profiles.iter().find(|p| p.id == id)
    }

    pub fn active_profile_mut(&mut self) -> Option<&mut Profile> {
        let id = self.active_profile_id.clone()?;
        self.profiles.iter_mut().find(|p| p.id == id)
    }

    pub fn is_active_admin(&self) -> bool {
        self.active_profile()
            .map(Profile::is_admin)
            .unwrap_or(false)
    }

    pub fn games_for_profile(&self, profile_id: &str) -> Vec<&GameEntry> {
        let mut games: Vec<&GameEntry> = self
            .games
            .iter()
            .filter(|g| g.is_visible_to(profile_id))
            .collect();
        games.sort_by(|a, b| {
            a.sort_order
                .cmp(&b.sort_order)
                .then_with(|| a.title().to_lowercase().cmp(&b.title().to_lowercase()))
        });
        games
    }

    pub fn game(&self, id: &str) -> Option<&GameEntry> {
        self.games.iter().find(|g| g.id == id)
    }

    pub fn game_mut(&mut self, id: &str) -> Option<&mut GameEntry> {
        self.games.iter_mut().find(|g| g.id == id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "settings_v1")]
    pub schema_version: u32,
    #[serde(default = "default_true")]
    pub animations: bool,
    /// Global background image, relative to the CouchDeck data directory.
    #[serde(default)]
    pub background: Option<PathBuf>,
    /// Preferred number of grid columns; `None` = automatic.
    #[serde(default)]
    pub columns: Option<u8>,
    #[serde(default = "default_true")]
    pub show_controller_hints: bool,
    #[serde(default = "default_true")]
    pub show_backgrounds: bool,
    /// UI language. `auto` follows the POSIX locale environment.
    #[serde(default)]
    pub language: LanguagePreference,
    /// Play a best-effort alert sound at time-budget warnings.
    #[serde(default = "default_true")]
    pub alarm_sound: bool,
    /// Extra administrator-configured `.desktop` roots. Absolute paths are
    /// preferred; relative paths are resolved by the caller.
    #[serde(default)]
    pub additional_desktop_dirs: Vec<PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: SETTINGS_SCHEMA_VERSION,
            animations: true,
            background: None,
            columns: None,
            show_controller_hints: true,
            show_backgrounds: true,
            language: LanguagePreference::Auto,
            alarm_sound: true,
            additional_desktop_dirs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AdminSecrets {
    #[serde(default = "secrets_v1")]
    pub schema_version: u32,
    /// profile id -> Argon2id PHC string. Never store plaintext PINs.
    #[serde(default)]
    pub pins: std::collections::BTreeMap<String, String>,
}

impl AdminSecrets {
    pub fn new() -> Self {
        Self {
            schema_version: SECRETS_SCHEMA_VERSION,
            pins: Default::default(),
        }
    }
}

/// Resolve an asset path stored in the library. Absolute paths are kept for
/// backwards compatibility; relative paths are resolved below the data dir.
pub fn resolve_data_path(data_dir: &Path, stored: &Path) -> PathBuf {
    if stored.is_absolute() {
        stored.to_path_buf()
    } else {
        data_dir.join(stored)
    }
}

pub fn new_id(prefix: &str, existing: &[String]) -> String {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let mut candidate = format!("{prefix}-{stamp}");
    let mut n = 2;
    while existing.iter().any(|id| id == &candidate) {
        candidate = format!("{prefix}-{stamp}-{n}");
        n += 1;
    }
    candidate
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_library_has_admin_and_profile() {
        let lib = Library::new();
        assert_eq!(lib.profiles.len(), 1);
        assert!(lib.is_active_admin());
        assert_eq!(lib.active_profile().unwrap().id, "admin");
    }

    #[test]
    fn visibility_rules() {
        let mut lib = Library::new();
        lib.profiles.push(Profile {
            id: "kid".into(),
            name: "Kind".into(),
            role: ProfileRole::Player,
            avatar: None,
            favorites: vec![],
            background: None,
            is_child: false,
            daily_limit_minutes: None,
        });
        lib.games.push(GameEntry {
            id: "g1".into(),
            desktop_file: PathBuf::from("/tmp/a.desktop"),
            title_override: Some("A".into()),
            desktop_name: None,
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
        lib.games.push(GameEntry {
            id: "g2".into(),
            desktop_file: PathBuf::from("/tmp/b.desktop"),
            title_override: Some("B".into()),
            desktop_name: None,
            desktop_icon: None,
            cover: None,
            background: None,
            visible_to_profiles: Some(vec!["admin".into()]),
            hidden: false,
            sort_order: 20,
            managed: false,
            created_at_unix: 0,
            desktop_id: None,
            source: Default::default(),
            launch_method: Default::default(),
        });
        assert_eq!(lib.games_for_profile("admin").len(), 2);
        assert_eq!(lib.games_for_profile("kid").len(), 1);
        lib.games[0].hidden = true;
        assert_eq!(lib.games_for_profile("admin").len(), 1);
    }

    #[test]
    fn v025_data_defaults_new_fields() {
        let profile: Profile =
            serde_json::from_str(r#"{"id":"old","name":"Old","role":"player","favorites":[]}"#)
                .unwrap();
        assert!(profile.background.is_none());
        assert!(!profile.is_child);
        assert!(profile.daily_limit_minutes.is_none());

        let settings: Settings =
            serde_json::from_str(r#"{"schema_version":1,"animations":true}"#).unwrap();
        assert_eq!(settings.language, LanguagePreference::Auto);
        assert!(settings.alarm_sound);
    }

    #[test]
    fn v031_data_defaults_040_game_fields() {
        let game: GameEntry = serde_json::from_str(
            r#"{"id":"old-game","desktop_file":"/tmp/old.desktop","created_at_unix":1}"#,
        )
        .unwrap();
        assert!(game.desktop_id.is_none());
        assert_eq!(game.source, GameSource::DesktopEntry);
        assert_eq!(game.launch_method, LaunchMethod::GioDesktop);

        let settings: Settings = serde_json::from_str(
            r#"{"schema_version":1,"animations":true,"language":"auto","alarm_sound":true}"#,
        )
        .unwrap();
        assert!(settings.additional_desktop_dirs.is_empty());
    }

    #[test]
    fn profile_background_and_child_fields_roundtrip() {
        let profile: Profile = serde_json::from_str(
            r#"{"id":"mia","name":"Mia","role":"player","background":"assets/backgrounds/mia.jpg","is_child":true,"daily_limit_minutes":60}"#,
        )
        .unwrap();
        assert_eq!(
            profile.background.as_deref(),
            Some(Path::new("assets/backgrounds/mia.jpg"))
        );
        assert!(profile.is_child);
        assert_eq!(profile.daily_limit_minutes, Some(60));
        let json = serde_json::to_string(&profile).unwrap();
        let again: Profile = serde_json::from_str(&json).unwrap();
        assert_eq!(again.background, profile.background);
        assert_eq!(again.daily_limit_minutes, profile.daily_limit_minutes);
    }

    #[test]
    fn normalize_repairs_duplicates_and_active_profile() {
        let mut lib = Library::new();
        lib.profiles.push(lib.profiles[0].clone());
        lib.active_profile_id = Some("does-not-exist".into());
        lib.normalize();
        assert!(lib.profiles[0].is_admin());
        assert_ne!(lib.profiles[0].id, lib.profiles[1].id);
        assert_eq!(lib.active_profile_id.as_deref(), Some("admin"));
    }
}
