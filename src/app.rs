//! Application state machine, navigation and user actions.

use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use eframe::egui;
use eframe::egui::{Key, Modifiers, ViewportCommand};

use crate::artwork::{self, ArtworkCache, ArtworkKind};
use crate::controller::{self, ControllerEvent, ControllerHandle, NavAction};
use crate::desktop::{self, DiscoveredEntry, DiscoveryResult, DiscoverySource, IconResolver};
use crate::i18n::{ArgValue, I18n, LanguagePreference};
use crate::launch::{self, LaunchOutcome};
use crate::logging::{log_error, log_info, log_warn};
use crate::model::{new_id, resolve_data_path, AdminSecrets, GameEntry, Library, Settings};
use crate::paths::XdgPaths;
use crate::storage::{self, LibraryLoad, SecretsLoad, SettingsLoad};
use crate::system::{SystemAction, SystemController};
use crate::timekeeping::{self, TimeStatus, TimeWarning, Timekeeping};

fn profile_switch_requires_parent(
    active_is_child: bool,
    active_id: Option<&str>,
    target_id: &str,
) -> bool {
    active_is_child && active_id.is_some_and(|active| active != target_id)
}

fn take_usage_seconds(delta: Duration, remainder: &mut Duration) -> u64 {
    let total = remainder.saturating_add(delta);
    let seconds = total.as_secs();
    *remainder = total.saturating_sub(Duration::from_secs(seconds));
    seconds
}

fn default_visibility_for_profiles(profiles: &[crate::model::Profile]) -> Option<Vec<String>> {
    if !profiles.iter().any(|profile| profile.is_child) {
        return None;
    }
    Some(
        profiles
            .iter()
            .filter(|profile| !profile.is_child)
            .map(|profile| profile.id.clone())
            .collect(),
    )
}

fn start_is_blocked(status: Option<&TimeStatus>) -> bool {
    status.is_some_and(|status| status.limited && status.exhausted)
}

fn admin_pin_matches(
    profiles: &[crate::model::Profile],
    secrets: &AdminSecrets,
    pin: &str,
) -> bool {
    profiles
        .iter()
        .filter(|profile| profile.is_admin() && !profile.is_child)
        .any(|profile| secrets.verify(&profile.id, pin))
}

pub const APP_NAME: &str = "CouchDeck";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Copy)]
pub struct AppOptions {
    pub windowed: bool,
    pub no_controller: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Library,
    Profiles,
    Admin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    Resume,
    Library,
    Profiles,
    Admin,
    Import,
    ToggleFullscreen,
    Minimize,
    ExtraTime,
    Reboot,
    PowerOff,
    About,
    Quit,
}

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub label: String,
    pub action: MenuAction,
}

pub struct MenuState {
    pub items: Vec<MenuItem>,
    pub focus: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmAction {
    RemoveGame(String),
    RemoveProfile(String),
    RemovePin(String),
    RepairSteam(String),
    Reboot,
    PowerOff,
    Quit,
}

pub struct ConfirmDialog {
    pub title: String,
    pub message: String,
    pub confirm_label: String,
    pub danger: bool,
    pub action: ConfirmAction,
    /// 0 = cancel, 1 = confirm. Destructive dialogs start on cancel.
    pub focus: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageSeverity {
    Info,
    Warning,
    Error,
}

pub struct MessageDialog {
    pub title: String,
    pub message: String,
    pub severity: MessageSeverity,
    pub close_label: String,
}

pub struct ImportState {
    pub entries: Vec<DiscoveredEntry>,
    pub selected: usize,
    pub focus_actions: bool,
    pub action_index: usize,
    pub error: Option<String>,
    pub loading: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickerPurpose {
    Cover(String),
    Background(String),
    GlobalBackground,
    Avatar(String),
    ProfileBackground(String),
    SearchDirectory,
    ImportDesktop,
}

impl PickerPurpose {
    pub fn title_key(&self) -> &'static str {
        match self {
            Self::Cover(_) => "picker-cover",
            Self::Background(_) => "picker-game-background",
            Self::GlobalBackground => "picker-global-background",
            Self::Avatar(_) => "picker-avatar",
            Self::ProfileBackground(_) => "picker-profile-background",
            Self::SearchDirectory => "picker-search-directory",
            Self::ImportDesktop => "picker-desktop",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PickerEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

pub struct FilePickerState {
    pub purpose: PickerPurpose,
    pub dir: PathBuf,
    pub entries: Vec<PickerEntry>,
    pub selected: usize,
    pub path_input: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParentAction {
    GrantExtraTime { profile_id: String },
    Minimize,
    Quit,
    Reboot,
    PowerOff,
    ActivateProfile(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PinPurpose {
    Unlock {
        profile_id: String,
    },
    Set {
        profile_id: String,
    },
    ConfirmSet {
        profile_id: String,
        first: String,
    },
    /// Verify any admin PIN for a single child-profile action without keeping
    /// the admin area unlocked.
    ParentAuth {
        action: ParentAction,
    },
}

pub struct PinPad {
    pub digits: String,
    pub purpose: PinPurpose,
    /// 0..=8 digits 1–9, 9 = backspace, 10 = 0, 11 = OK.
    pub focus: usize,
    pub error: Option<String>,
}

impl PinPad {
    pub fn title_key(&self) -> &'static str {
        match self.purpose {
            PinPurpose::Unlock { .. } => "pin-title-unlock",
            PinPurpose::Set { .. } => "pin-title-set",
            PinPurpose::ConfirmSet { .. } => "pin-title-confirm",
            PinPurpose::ParentAuth { .. } => "pin-title-parent",
        }
    }

    pub fn masked(&self) -> String {
        "*".repeat(self.digits.len())
    }
}

pub struct TimeExpiredState {
    pub profile_id: String,
    /// 0 = request extra time, 1 = back to library.
    pub focus: usize,
}

pub struct ExtraTimeDialog {
    pub profile_id: String,
    pub focus: usize,
    pub custom_mode: bool,
    pub custom_minutes: u32,
    pub error: Option<String>,
}

pub enum Overlay {
    None,
    Menu(MenuState),
    Import(ImportState),
    Picker(FilePickerState),
    Pin(PinPad),
    Confirm(ConfirmDialog),
    Message(MessageDialog),
    TimeExpired(TimeExpiredState),
    ExtraTime(ExtraTimeDialog),
}

pub struct LaunchState {
    pub title: String,
    pub started: Instant,
    pub outcome: Option<LaunchOutcome>,
    pub completed_at: Option<Instant>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Warning,
    Error,
}

pub struct Toast {
    pub message: String,
    pub kind: ToastKind,
    pub created_at: Instant,
    pub duration: Duration,
}

pub struct FatalState {
    pub message: String,
    pub backup: Option<PathBuf>,
    pub focus: usize,
}

#[derive(Debug, Clone)]
pub enum AdminView {
    Menu,
    Games,
    GameEdit(String),
    Profiles,
    Discovery,
    Integrations,
    Settings,
    Pin,
}

pub struct AdminState {
    pub view: AdminView,
    pub selected_game: Option<String>,
    pub selected_profile: Option<String>,
    pub selected_steam_game: Option<String>,
    pub selected_instance: Option<String>,
    pub prism_title: String,
    pub needs_initial_focus: bool,
    /// Controller focus movement is applied after the admin UI has collected
    /// its focusable widgets for this frame.
    pub pending_steps: i32,
    /// Controller confirm is injected at the beginning of the next frame, after
    /// any pending focus movement has been applied.
    pub pending_confirm: bool,
}

impl Default for AdminState {
    fn default() -> Self {
        Self {
            view: AdminView::Menu,
            selected_game: None,
            selected_profile: None,
            selected_steam_game: None,
            selected_instance: None,
            prism_title: String::new(),
            needs_initial_focus: true,
            pending_steps: 0,
            pending_confirm: false,
        }
    }
}

pub struct CouchDeckApp {
    pub paths: XdgPaths,
    pub library: Library,
    pub settings: Settings,
    pub i18n: I18n,
    pub secrets: AdminSecrets,
    pub controller: ControllerHandle,
    pub controllers: Vec<String>,
    pub controller_error: Option<String>,
    pub controller_ready: bool,
    pub artwork: ArtworkCache,
    pub icons: IconResolver,
    pub screen: Screen,
    pub overlay: Overlay,
    pub focus: usize,
    pub profile_focus: usize,
    pub menu_focus: usize,
    pub admin: AdminState,
    pub admin_unlocked: bool,
    /// View to open after a successful PIN unlock (e.g. direct import from the
    /// library). `None` means the admin main menu.
    pub pending_admin_view: Option<AdminView>,
    pub launch: Option<LaunchState>,
    pub launch_rx: Option<Receiver<LaunchOutcome>>,
    pub last_launch_started: Option<Instant>,
    pub system: SystemController,
    pub timekeeping: Timekeeping,
    pub discovery: Option<DiscoveryResult>,
    pub discovery_rx: Option<Receiver<DiscoveryResult>>,
    pub discovery_loading: bool,
    pub last_time_tick: Instant,
    pub last_time_save: Instant,
    pub time_remainder: Duration,
    pub last_budget_exhausted: bool,
    pub time_expired_pending: bool,
    /// Set only after a parent-authorized minimisation or application close.
    pub allow_child_minimize: bool,
    pub allow_close: bool,
    /// Set when leaving fullscreen so the minimised command is sent on the next
    /// frame, after the compositor has processed the fullscreen exit.
    pub pending_minimize: bool,
    pub window_focused: Option<bool>,
    /// Drop queued controller navigation for the first frame after the window
    /// regains focus, so buffered input from a running game cannot leak in.
    pub discard_controller_nav: bool,
    pub toast: Option<Toast>,
    pub fatal: Option<FatalState>,
    pub fullscreen: bool,
    pub style_scale: f32,
    pub focus_changed_at: Instant,
    pub should_quit: bool,
    pub gio_message: Option<String>,
    pub frame_count: u64,
}

impl CouchDeckApp {
    pub fn new(cc: &eframe::CreationContext<'_>, paths: XdgPaths, options: AppOptions) -> Self {
        let mut notices: Vec<String> = Vec::new();
        let mut fatal = None;

        let library = match storage::load_library(&paths) {
            LibraryLoad::Missing => {
                let library = Library::new();
                if let Err(err) = storage::save_library(&paths, &library) {
                    notices.push(format!(
                        "Die erste Bibliothek konnte nicht gespeichert werden: {err}"
                    ));
                }
                library
            }
            LibraryLoad::Loaded(library) => library,
            LibraryLoad::Corrupt { message, backup } => {
                fatal = Some(FatalState {
                    message,
                    backup,
                    focus: 1,
                });
                Library::new()
            }
        };
        let settings = match storage::load_settings(&paths) {
            SettingsLoad::Missing => Settings::default(),
            SettingsLoad::Loaded(settings) => settings,
            SettingsLoad::Corrupt { message, backup } => {
                notices.push(match backup {
                    Some(path) => format!(
                        "{message}\n\nEine Sicherung wurde angelegt: {}",
                        path.display()
                    ),
                    None => message,
                });
                Settings::default()
            }
        };
        let i18n = I18n::new(settings.language);
        let timekeeping = match timekeeping::load(&paths) {
            timekeeping::TimekeepingLoad::Missing => Timekeeping::new(),
            timekeeping::TimekeepingLoad::Loaded(mut value) => {
                value.normalize();
                value
            }
            timekeeping::TimekeepingLoad::Corrupt { message, backup } => {
                notices.push(match backup {
                    Some(path) => format!(
                        "{message}\n\nEine Sicherung wurde angelegt: {}",
                        path.display()
                    ),
                    None => message,
                });
                Timekeeping::new()
            }
        };
        let secrets = match storage::load_secrets(&paths) {
            SecretsLoad::Missing => AdminSecrets::new(),
            SecretsLoad::Loaded(secrets) => secrets,
            SecretsLoad::Corrupt { message, backup } => {
                notices.push(match backup {
                    Some(path) => {
                        format!("{message}\n\nSicherung der PIN-Datei: {}", path.display())
                    }
                    None => message,
                });
                AdminSecrets::new()
            }
        };

        let gio_message = match launch::gio_version() {
            Ok(version) => {
                log_info("launch", format!("GIO: {version}"));
                None
            }
            Err(err) => {
                log_error("launch", &err);
                notices.push(format!(
                    "GIO wurde nicht gefunden. Spiele können nicht gestartet werden.\n{err}"
                ));
                Some(err)
            }
        };

        let controller = controller::spawn(cc.egui_ctx.clone(), !options.no_controller);
        let icons = IconResolver::new(&paths);
        let active_index = library
            .active_profile()
            .and_then(|active| library.profiles.iter().position(|p| p.id == active.id))
            .unwrap_or(0);

        let mut app = Self {
            paths,
            library,
            settings,
            i18n,
            secrets,
            controller,
            controllers: Vec::new(),
            controller_error: None,
            controller_ready: false,
            artwork: ArtworkCache::new(),
            icons,
            screen: Screen::Library,
            overlay: Overlay::None,
            focus: 0,
            profile_focus: active_index,
            menu_focus: 0,
            admin: AdminState::default(),
            admin_unlocked: false,
            pending_admin_view: None,
            launch: None,
            launch_rx: None,
            last_launch_started: None,
            system: SystemController::new(),
            timekeeping,
            discovery: None,
            discovery_rx: None,
            discovery_loading: false,
            last_time_tick: Instant::now(),
            last_time_save: Instant::now(),
            time_remainder: Duration::ZERO,
            last_budget_exhausted: false,
            time_expired_pending: false,
            allow_child_minimize: false,
            allow_close: false,
            pending_minimize: false,
            window_focused: None,
            discard_controller_nav: false,
            toast: None,
            fatal,
            fullscreen: !options.windowed,
            style_scale: 0.0,
            focus_changed_at: Instant::now(),
            should_quit: false,
            gio_message,
            frame_count: 0,
        };

        if app.fatal.is_none() && !notices.is_empty() {
            app.overlay = Overlay::Message(MessageDialog {
                title: app.t("app-startup-notice-title"),
                message: notices.join("\n\n"),
                severity: MessageSeverity::Warning,
                close_label: app.t("common-continue"),
            });
        }
        log_info(
            "app",
            format!(
                "Start: {} Profile, {} Spiele, Daten: {}",
                app.library.profiles.len(),
                app.library.games.len(),
                app.paths.data_dir().display()
            ),
        );
        log_info(
            "app",
            format!(
                "Hintergrund: {:?}, anzeigen={}",
                app.settings.background, app.settings.show_backgrounds
            ),
        );
        app.start_discovery();
        app
    }

    // ---------------------------------------------------------------------
    // Localization
    // ---------------------------------------------------------------------

    pub fn t(&self, key: &str) -> String {
        self.i18n.text(key)
    }

    pub fn tf(&self, key: &str, values: &[(&str, ArgValue)]) -> String {
        self.i18n.format(key, values)
    }

    pub fn set_language(&mut self, preference: LanguagePreference) {
        if self.settings.language == preference {
            return;
        }
        self.settings.language = preference;
        self.i18n.set_preference(preference);
        self.save_settings();
    }

    pub fn start_discovery(&mut self) {
        if self.discovery_loading {
            return;
        }
        let paths = self.paths.clone();
        let additional = self.settings.additional_desktop_dirs.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = desktop::discover_entries(&paths, &additional);
            let _ = tx.send(result);
        });
        self.discovery_rx = Some(rx);
        self.discovery_loading = true;
    }

    fn poll_discovery(&mut self) {
        let Some(rx) = &self.discovery_rx else {
            return;
        };
        let result = match rx.try_recv() {
            Ok(result) => result,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.discovery_rx = None;
                self.discovery_loading = false;
                self.notify_error("Discovery-Thread wurde unerwartet beendet.");
                return;
            }
        };
        self.discovery_rx = None;
        self.discovery_loading = false;
        log_info(
            "discovery",
            format!(
                "{} Desktop-Einträge, {} Quellen, {} Warnungen",
                result.entries.len(),
                result.roots.len(),
                result.warnings.len()
            ),
        );
        let entries = result.entries.clone();
        self.discovery = Some(result);
        if let Overlay::Import(state) = &mut self.overlay {
            state.entries = entries;
            state.selected = 0;
            state.loading = false;
        }
    }

    // ---------------------------------------------------------------------
    // Data helpers
    // ---------------------------------------------------------------------

    pub fn active_profile(&self) -> Option<&crate::model::Profile> {
        self.library.active_profile()
    }

    pub fn active_profile_id(&self) -> Option<&str> {
        self.library.active_profile().map(|p| p.id.as_str())
    }

    pub fn is_admin(&self) -> bool {
        // A profile marked as a child profile never gets admin UI access, even
        // if a hand-edited role says otherwise.
        self.active_profile()
            .map(|profile| profile.is_admin() && !profile.is_child)
            .unwrap_or(false)
    }

    pub fn is_active_child(&self) -> bool {
        self.active_profile()
            .map(|profile| profile.is_child)
            .unwrap_or(false)
    }

    pub fn visible_games(&self) -> Vec<GameEntry> {
        match self.library.active_profile() {
            Some(profile) => self
                .library
                .games_for_profile(&profile.id)
                .into_iter()
                .cloned()
                .collect(),
            None => Vec::new(),
        }
    }

    pub fn focused_game(&self) -> Option<GameEntry> {
        self.visible_games().get(self.focus).cloned()
    }

    pub fn resolve_data_path(&self, stored: &Path) -> PathBuf {
        resolve_data_path(&self.paths.data_dir(), stored)
    }

    pub fn game_cover_path(&self, game: &GameEntry) -> Option<PathBuf> {
        game.cover.as_ref().map(|p| self.resolve_data_path(p))
    }

    pub fn game_background_path(&self, game: &GameEntry) -> Option<PathBuf> {
        game.background.as_ref().map(|p| self.resolve_data_path(p))
    }

    pub fn global_background_path(&self) -> Option<PathBuf> {
        self.settings
            .background
            .as_ref()
            .map(|p| self.resolve_data_path(p))
    }

    pub fn profile_avatar_path(&self, profile: &crate::model::Profile) -> Option<PathBuf> {
        profile.avatar.as_ref().map(|p| self.resolve_data_path(p))
    }

    pub fn profile_background_path(&self, profile: &crate::model::Profile) -> Option<PathBuf> {
        profile
            .background
            .as_ref()
            .map(|p| self.resolve_data_path(p))
    }

    pub fn active_profile_background_path(&self) -> Option<PathBuf> {
        self.active_profile()
            .and_then(|profile| self.profile_background_path(profile))
    }

    pub fn time_status_for(&mut self, profile: &crate::model::Profile) -> TimeStatus {
        let today = timekeeping::today_string();
        self.timekeeping
            .status(&profile.id, profile.daily_limit_minutes, &today)
    }

    pub fn active_time_status(&mut self) -> Option<TimeStatus> {
        let profile = self.active_profile().cloned()?;
        profile.is_child.then(|| self.time_status_for(&profile))
    }

    /// Central gate for every new game/media launch. `launch_game` calls this;
    /// future media launchers must do the same.
    pub fn check_start_allowed(&mut self) -> bool {
        if start_is_blocked(self.active_time_status().as_ref()) {
            self.time_expired_pending = true;
            self.maybe_show_time_expired();
            return false;
        }
        true
    }

    fn save_timekeeping(&mut self) {
        if let Err(err) = timekeeping::save(&self.paths, &self.timekeeping) {
            log_error("timekeeping", &err);
            self.notify_error(err);
        }
        self.last_time_save = Instant::now();
    }

    fn tick_time_budget(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        let delta = now.saturating_duration_since(self.last_time_tick);
        self.last_time_tick = now;

        let Some(profile) = self.active_profile().cloned() else {
            self.last_budget_exhausted = false;
            self.time_remainder = Duration::ZERO;
            return;
        };
        if !profile.is_child || profile.daily_limit_minutes.is_none() {
            self.last_budget_exhausted = false;
            self.time_remainder = Duration::ZERO;
            return;
        }

        let elapsed_seconds = take_usage_seconds(delta, &mut self.time_remainder);
        let today = timekeeping::today_string();
        let status = self.timekeeping.add_usage(
            &profile.id,
            elapsed_seconds,
            profile.daily_limit_minutes,
            &today,
        );
        let warning =
            self.timekeeping
                .warning_to_emit(&profile.id, profile.daily_limit_minutes, &today);

        if let Some(warning) = warning {
            let message = match warning {
                TimeWarning::FiveMinutes => self.t("time-warning-five"),
                TimeWarning::OneMinute => self.t("time-warning-one"),
                TimeWarning::Exhausted => self.t("time-warning-exhausted"),
            };
            self.notify_warning(message);
            if self.settings.alarm_sound && warning != TimeWarning::Exhausted {
                crate::sound::play_alert();
            }
            self.save_timekeeping();
        }

        if status.exhausted {
            if !self.last_budget_exhausted {
                self.last_budget_exhausted = true;
                self.time_expired_pending = true;
                if self.settings.alarm_sound {
                    crate::sound::play_alert();
                }
            }
        } else {
            self.last_budget_exhausted = false;
            if matches!(self.overlay, Overlay::TimeExpired(_)) {
                self.overlay = Overlay::None;
            }
        }

        if self.last_time_save.elapsed() >= Duration::from_secs(30) {
            self.save_timekeeping();
        }
        self.maybe_show_time_expired();
        if !status.exhausted {
            ctx.request_repaint_after(Duration::from_secs(5));
        }
    }

    fn maybe_show_time_expired(&mut self) {
        if !self.time_expired_pending
            || self.fatal.is_some()
            || self.launch.is_some()
            || self.system.is_active()
            || !matches!(self.overlay, Overlay::None)
        {
            return;
        }
        let Some(profile) = self.active_profile().cloned() else {
            return;
        };
        if !profile.is_child {
            self.time_expired_pending = false;
            return;
        }
        self.overlay = Overlay::TimeExpired(TimeExpiredState {
            profile_id: profile.id,
            focus: 0,
        });
        self.time_expired_pending = false;
    }

    pub fn grant_extra_minutes(&mut self, profile_id: &str, minutes: u32) -> Result<(), String> {
        if minutes == 0 || minutes > 240 {
            return Err(self.t("app-extra-invalid"));
        }
        let Some(profile) = self
            .library
            .profiles
            .iter()
            .find(|profile| profile.id == profile_id)
        else {
            return Err(self.t("app-profile-not-found"));
        };
        if !profile.is_child {
            return Err(self.t("app-extra-child-only"));
        }
        let name = profile.name.clone();
        let today = timekeeping::today_string();
        self.timekeeping
            .add_extra_minutes(profile_id, minutes, &today);
        self.save_timekeeping();
        self.last_budget_exhausted = false;
        self.time_expired_pending = false;
        self.last_time_tick = Instant::now();
        self.time_remainder = Duration::ZERO;
        self.overlay = Overlay::None;
        self.screen = Screen::Library;
        let message = self.tf(
            "app-extra-granted",
            &[("minutes", minutes.into()), ("name", name.into())],
        );
        self.notify(message);
        Ok(())
    }

    pub fn reset_extra_minutes(&mut self, profile_id: &str) {
        let today = timekeeping::today_string();
        self.timekeeping.reset_extra_minutes(profile_id, &today);
        self.save_timekeeping();
        self.last_time_tick = Instant::now();
        self.notify_key("app-extra-reset");
    }

    pub fn save_library(&mut self) {
        if self.fatal.is_some() {
            return;
        }
        if let Err(err) = storage::save_library(&self.paths, &self.library) {
            log_error("storage", &err);
            self.notify_error(err);
        }
    }

    pub fn save_settings(&mut self) {
        if let Err(err) = storage::save_settings(&self.paths, &self.settings) {
            log_error("storage", &err);
            self.notify_error(err);
        }
    }

    pub fn save_secrets(&mut self) {
        if let Err(err) = storage::save_secrets(&self.paths, &self.secrets) {
            log_error("storage", &err);
            self.notify_error(err);
        }
    }

    pub fn notify(&mut self, message: impl Into<String>) {
        self.toast = Some(Toast {
            message: message.into(),
            kind: ToastKind::Info,
            created_at: Instant::now(),
            duration: Duration::from_secs(4),
        });
    }

    pub fn notify_warning(&mut self, message: impl Into<String>) {
        self.toast = Some(Toast {
            message: message.into(),
            kind: ToastKind::Warning,
            created_at: Instant::now(),
            duration: Duration::from_secs(6),
        });
    }

    pub fn notify_error(&mut self, message: impl Into<String>) {
        self.toast = Some(Toast {
            message: message.into(),
            kind: ToastKind::Error,
            created_at: Instant::now(),
            duration: Duration::from_secs(8),
        });
    }

    pub fn notify_key(&mut self, key: &str) {
        let message = self.t(key);
        self.notify(message);
    }

    pub fn notify_warning_key(&mut self, key: &str) {
        let message = self.t(key);
        self.notify_warning(message);
    }

    pub fn message_key(&mut self, title_key: &str, message_key: &str, severity: MessageSeverity) {
        let title = self.t(title_key);
        let message = self.t(message_key);
        self.message_dialog(title, message, severity);
    }

    pub fn message_dialog(
        &mut self,
        title: impl Into<String>,
        message: impl Into<String>,
        severity: MessageSeverity,
    ) {
        self.overlay = Overlay::Message(MessageDialog {
            title: title.into(),
            message: message.into(),
            severity,
            close_label: self.t("common-close"),
        });
    }

    // ---------------------------------------------------------------------
    // Focus / screens
    // ---------------------------------------------------------------------

    pub fn column_count(&self, ctx: &egui::Context) -> usize {
        let rect = ctx.screen_rect();
        let scale = crate::ui::ui_scale(ctx);
        let available = (rect.width() - 96.0 * scale).max(320.0);
        let automatic = ((available / (245.0 * scale)).floor() as usize).clamp(3, 6);
        self.settings
            .columns
            .map(|c| (c as usize).clamp(3, 6))
            .unwrap_or(automatic)
    }

    pub fn set_focus(&mut self, index: usize, count: usize) {
        let new_focus = if count == 0 { 0 } else { index.min(count - 1) };
        if new_focus != self.focus {
            self.focus = new_focus;
            self.focus_changed_at = Instant::now();
        }
    }

    pub fn move_library_focus(&mut self, action: NavAction, columns: usize) {
        let count = self.visible_games().len();
        if count == 0 {
            return;
        }
        let columns = columns.max(1);
        let current = self.focus.min(count - 1);
        let next = match action {
            NavAction::Left => current.saturating_sub(1),
            NavAction::Right => (current + 1).min(count - 1),
            NavAction::Up => current.saturating_sub(columns),
            NavAction::Down => (current + columns).min(count - 1),
            _ => current,
        };
        self.set_focus(next, count);
    }

    pub fn open_profiles(&mut self) {
        self.profile_focus = self
            .active_profile()
            .and_then(|active| self.library.profiles.iter().position(|p| p.id == active.id))
            .unwrap_or(0);
        // Leaving the admin screen through the profile picker ends the
        // unlocked admin session.
        self.admin_unlocked = false;
        self.pending_admin_view = None;
        self.screen = Screen::Profiles;
        self.overlay = Overlay::None;
    }

    pub fn request_activate_profile(&mut self, index: usize) {
        let Some(target) = self.library.profiles.get(index) else {
            return;
        };
        let target_id = target.id.clone();
        if profile_switch_requires_parent(
            self.is_active_child(),
            self.active_profile_id(),
            &target_id,
        ) {
            self.request_parent_action(ParentAction::ActivateProfile(target_id));
            return;
        }
        self.activate_profile_now(index);
    }

    fn activate_profile_now(&mut self, index: usize) {
        let Some(profile) = self.library.profiles.get(index) else {
            return;
        };
        let id = profile.id.clone();
        let name = profile.name.clone();
        self.library.active_profile_id = Some(id);
        self.focus = 0;
        self.profile_focus = index;
        self.admin_unlocked = false;
        self.pending_admin_view = None;
        self.last_time_tick = Instant::now();
        self.time_remainder = Duration::ZERO;
        self.last_budget_exhausted = false;
        self.time_expired_pending = false;
        self.allow_child_minimize = false;
        self.allow_close = false;
        self.screen = Screen::Library;
        self.overlay = Overlay::None;
        self.save_library();
        self.save_timekeeping();
        self.notify(self.tf("app-profile-active", &[("name", name.into())]));
    }

    pub fn has_admin_pin_configured(&self) -> bool {
        self.has_any_admin_pin()
    }

    fn has_any_admin_pin(&self) -> bool {
        self.library
            .profiles
            .iter()
            .filter(|profile| profile.is_admin() && !profile.is_child)
            .any(|profile| self.secrets.has_pin(&profile.id))
    }

    fn verify_any_admin_pin(&self, pin: &str) -> bool {
        admin_pin_matches(&self.library.profiles, &self.secrets, pin)
    }

    fn request_parent_action(&mut self, action: ParentAction) {
        if !self.has_any_admin_pin() {
            self.message_key(
                "app-no-admin-pin-title",
                "app-no-admin-pin-message",
                MessageSeverity::Warning,
            );
            return;
        }
        self.overlay = Overlay::Pin(PinPad {
            digits: String::new(),
            purpose: PinPurpose::ParentAuth { action },
            focus: 0,
            error: None,
        });
    }

    fn execute_parent_action(&mut self, ctx: &egui::Context, action: ParentAction) {
        match action {
            ParentAction::GrantExtraTime { profile_id } => {
                self.overlay = Overlay::ExtraTime(ExtraTimeDialog {
                    profile_id,
                    focus: 0,
                    custom_mode: false,
                    custom_minutes: 15,
                    error: None,
                });
            }
            ParentAction::Minimize => {
                self.allow_child_minimize = true;
                self.minimize_to_desktop(ctx);
            }
            ParentAction::Quit => {
                self.overlay = Overlay::Confirm(ConfirmDialog {
                    title: self.t("confirm-quit-title"),
                    message: self.t("confirm-quit-message"),
                    confirm_label: self.t("confirm-quit-label"),
                    danger: false,
                    action: ConfirmAction::Quit,
                    focus: 0,
                });
            }
            ParentAction::Reboot => {
                self.overlay = Overlay::Confirm(ConfirmDialog {
                    title: self.t("confirm-reboot-title"),
                    message: self.t("confirm-power-message"),
                    confirm_label: self.t("confirm-reboot-label"),
                    danger: true,
                    action: ConfirmAction::Reboot,
                    focus: 0,
                });
            }
            ParentAction::PowerOff => {
                self.overlay = Overlay::Confirm(ConfirmDialog {
                    title: self.t("confirm-poweroff-title"),
                    message: self.t("confirm-power-message"),
                    confirm_label: self.t("confirm-poweroff-label"),
                    danger: true,
                    action: ConfirmAction::PowerOff,
                    focus: 0,
                });
            }
            ParentAction::ActivateProfile(profile_id) => {
                if let Some(index) = self
                    .library
                    .profiles
                    .iter()
                    .position(|profile| profile.id == profile_id)
                {
                    self.activate_profile_now(index);
                }
            }
        }
    }

    pub fn request_extra_time(&mut self) {
        let Some(profile) = self.active_profile().cloned() else {
            return;
        };
        if !profile.is_child {
            self.notify_warning_key("app-extra-child-only");
            return;
        }
        self.request_parent_action(ParentAction::GrantExtraTime {
            profile_id: profile.id,
        });
    }

    pub fn open_admin(&mut self) {
        self.open_admin_view(AdminView::Menu);
    }

    pub fn open_admin_games(&mut self) {
        self.open_admin_view(AdminView::Games);
    }

    fn open_admin_view(&mut self, view: AdminView) {
        if !self.is_admin() {
            self.notify_warning_key("app-admin-only");
            return;
        }
        let Some(profile_id) = self.active_profile_id().map(ToOwned::to_owned) else {
            return;
        };
        if self.secrets.has_pin(&profile_id) && !self.admin_unlocked {
            self.pending_admin_view = Some(view);
            self.overlay = Overlay::Pin(PinPad {
                digits: String::new(),
                purpose: PinPurpose::Unlock { profile_id },
                focus: 0,
                error: None,
            });
        } else {
            self.pending_admin_view = None;
            self.enter_admin(view);
        }
    }

    fn enter_admin(&mut self, view: AdminView) {
        self.admin_unlocked = true;
        self.screen = Screen::Admin;
        self.admin = AdminState::default();
        self.admin.view = view.clone();
        self.admin.needs_initial_focus = true;
        match view {
            AdminView::Games => {
                self.admin.selected_game = self.library.games.first().map(|game| game.id.clone());
            }
            AdminView::Profiles => {
                self.admin.selected_profile = self
                    .active_profile_id()
                    .map(ToOwned::to_owned)
                    .or_else(|| self.library.profiles.first().map(|p| p.id.clone()));
            }
            _ => {}
        }
        self.overlay = Overlay::None;
    }

    pub fn leave_admin(&mut self) {
        self.screen = Screen::Library;
        self.admin_unlocked = false;
        self.pending_admin_view = None;
        self.admin.view = AdminView::Menu;
        self.admin.pending_steps = 0;
        self.admin.pending_confirm = false;
        self.overlay = Overlay::None;
    }

    pub fn admin_back(&mut self) {
        self.admin.pending_steps = 0;
        self.admin.pending_confirm = false;
        match self.admin.view {
            AdminView::Menu => self.leave_admin(),
            _ => {
                self.admin.view = AdminView::Menu;
                self.admin.needs_initial_focus = true;
            }
        }
    }

    pub fn open_menu(&mut self) {
        let mut items = Vec::new();
        if self.screen == Screen::Library {
            items.push(MenuItem {
                label: self.t("menu-resume"),
                action: MenuAction::Resume,
            });
        } else {
            items.push(MenuItem {
                label: self.t("menu-back-library"),
                action: MenuAction::Library,
            });
        }
        items.push(MenuItem {
            label: self.t("menu-profiles"),
            action: MenuAction::Profiles,
        });
        if self
            .active_profile()
            .map(|profile| profile.is_child && profile.daily_limit_minutes.is_some())
            .unwrap_or(false)
        {
            items.push(MenuItem {
                label: self.t("menu-extra-time"),
                action: MenuAction::ExtraTime,
            });
        }
        if self.is_admin() {
            if self.screen != Screen::Admin {
                items.push(MenuItem {
                    label: self.t("menu-admin"),
                    action: MenuAction::Admin,
                });
            }
            items.push(MenuItem {
                label: self.t("menu-import"),
                action: MenuAction::Import,
            });
        }
        items.push(MenuItem {
            label: if self.fullscreen {
                self.t("menu-fullscreen-exit")
            } else {
                self.t("menu-fullscreen-enter")
            },
            action: MenuAction::ToggleFullscreen,
        });
        items.push(MenuItem {
            label: self.t("menu-minimize"),
            action: MenuAction::Minimize,
        });
        items.push(MenuItem {
            label: self.t("menu-reboot"),
            action: MenuAction::Reboot,
        });
        items.push(MenuItem {
            label: self.t("menu-poweroff"),
            action: MenuAction::PowerOff,
        });
        items.push(MenuItem {
            label: self.t("menu-about"),
            action: MenuAction::About,
        });
        items.push(MenuItem {
            label: self.t("menu-quit"),
            action: MenuAction::Quit,
        });
        self.menu_focus = 0;
        self.overlay = Overlay::Menu(MenuState {
            items,
            focus: self.menu_focus,
        });
    }

    pub fn execute_menu_action(&mut self, ctx: &egui::Context, action: MenuAction) {
        match action {
            MenuAction::Resume => self.overlay = Overlay::None,
            MenuAction::Library => {
                if self.screen == Screen::Admin {
                    self.admin_unlocked = false;
                    self.pending_admin_view = None;
                }
                self.screen = Screen::Library;
                self.overlay = Overlay::None;
            }
            MenuAction::Profiles => self.open_profiles(),
            MenuAction::Admin => {
                self.overlay = Overlay::None;
                self.open_admin();
            }
            MenuAction::Import => {
                self.overlay = Overlay::None;
                self.open_import();
            }
            MenuAction::ToggleFullscreen => {
                self.fullscreen = !self.fullscreen;
                ctx.send_viewport_cmd(ViewportCommand::Decorations(!self.fullscreen));
                ctx.send_viewport_cmd(ViewportCommand::Fullscreen(self.fullscreen));
                self.overlay = Overlay::None;
            }
            MenuAction::Minimize => self.request_minimize(ctx),
            MenuAction::ExtraTime => {
                self.overlay = Overlay::None;
                self.request_extra_time();
            }
            MenuAction::Reboot => {
                if self.is_active_child() {
                    self.request_parent_action(ParentAction::Reboot);
                } else {
                    self.overlay = Overlay::Confirm(ConfirmDialog {
                        title: self.t("confirm-reboot-title"),
                        message: self.t("confirm-power-message"),
                        confirm_label: self.t("confirm-reboot-label"),
                        danger: true,
                        action: ConfirmAction::Reboot,
                        focus: 0,
                    });
                }
            }
            MenuAction::PowerOff => {
                if self.is_active_child() {
                    self.request_parent_action(ParentAction::PowerOff);
                } else {
                    self.overlay = Overlay::Confirm(ConfirmDialog {
                        title: self.t("confirm-poweroff-title"),
                        message: self.t("confirm-power-message"),
                        confirm_label: self.t("confirm-poweroff-label"),
                        danger: true,
                        action: ConfirmAction::PowerOff,
                        focus: 0,
                    });
                }
            }
            MenuAction::About => {
                let gio = self
                    .gio_message
                    .as_deref()
                    .map(|e| self.tf("menu-gio-problem", &[("error", e.into())]))
                    .unwrap_or_else(|| self.t("menu-gio-ok"));
                let data = self.paths.data_dir().display().to_string();
                let settings = self.paths.settings_path().display().to_string();
                let cache = self.paths.cache_dir().display().to_string();
                let log = self.paths.log_path().display().to_string();
                self.overlay = Overlay::Message(MessageDialog {
                    title: format!("{APP_NAME} {APP_VERSION}"),
                    message: self.tf(
                        "menu-about-text",
                        &[
                            ("data", data.into()),
                            ("settings", settings.into()),
                            ("cache", cache.into()),
                            ("log", log.into()),
                            ("gio", gio.into()),
                        ],
                    ),
                    severity: MessageSeverity::Info,
                    close_label: self.t("common-close"),
                });
            }
            MenuAction::Quit => {
                if self.is_active_child() {
                    self.request_parent_action(ParentAction::Quit);
                } else {
                    self.overlay = Overlay::Confirm(ConfirmDialog {
                        title: self.t("confirm-quit-title"),
                        message: self.t("confirm-quit-message"),
                        confirm_label: self.t("confirm-quit-label"),
                        danger: false,
                        action: ConfirmAction::Quit,
                        focus: 0,
                    });
                }
            }
        }
    }

    // ---------------------------------------------------------------------
    // Import / artwork / profiles
    // ---------------------------------------------------------------------

    pub fn open_import(&mut self) {
        if !self.is_admin() {
            self.notify_warning_key("app-admin-import-only");
            return;
        }
        if let Some(profile_id) = self.active_profile_id() {
            if self.secrets.has_pin(profile_id) && !self.admin_unlocked {
                self.notify_warning(self.t("app-admin-import-hint"));
                self.open_admin_games();
                return;
            }
        }
        let entries = self
            .discovery
            .as_ref()
            .map(|discovery| discovery.entries.clone())
            .unwrap_or_default();
        let loading = self.discovery.is_none() || self.discovery_loading;
        if self.discovery.is_none() {
            self.start_discovery();
        }
        self.overlay = Overlay::Import(ImportState {
            entries,
            selected: 0,
            focus_actions: false,
            action_index: 0,
            error: None,
            loading,
        });
    }

    pub fn open_import_file_picker(&mut self) {
        self.open_picker(PickerPurpose::ImportDesktop);
    }

    pub fn load_import_file(&mut self, path: PathBuf) {
        match desktop::parse_desktop_file(&path) {
            Ok(entry) => {
                let desktop_id = entry.file_name().trim_end_matches(".desktop").to_owned();
                let source_dir = path
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| self.paths.home.clone());
                self.overlay = Overlay::Import(ImportState {
                    entries: vec![DiscoveredEntry {
                        file: entry,
                        desktop_id,
                        source: DiscoverySource::Manual,
                        source_dir,
                        menu_visible: true,
                    }],
                    selected: 0,
                    focus_actions: false,
                    action_index: 0,
                    error: None,
                    loading: false,
                });
            }
            Err(err) => {
                let title = self.t("app-desktop-invalid-title");
                let message = self.tf(
                    "app-desktop-invalid-message",
                    &[("error", err.to_string().into())],
                );
                self.message_dialog(title, message, MessageSeverity::Error);
            }
        }
    }

    pub fn import_entry(&mut self, entry: &DiscoveredEntry) {
        if !self.is_admin() {
            self.notify_warning_key("app-import-not-admin");
            return;
        }
        let canonical =
            std::fs::canonicalize(&entry.file.path).unwrap_or_else(|_| entry.file.path.clone());
        let duplicate = self.library.games.iter().any(|game| {
            let existing = std::fs::canonicalize(&game.desktop_file)
                .unwrap_or_else(|_| game.desktop_file.clone());
            existing == canonical
        });
        if duplicate {
            self.overlay = Overlay::None;
            self.notify_warning_key("app-already-imported");
            return;
        }
        let ids: Vec<String> = self.library.games.iter().map(|g| g.id.clone()).collect();
        let id = new_id("game", &ids);
        let max_sort = self
            .library
            .games
            .iter()
            .map(|g| g.sort_order)
            .max()
            .unwrap_or(0);
        let managed = entry.source == DiscoverySource::Managed
            || entry
                .file
                .path
                .starts_with(self.paths.managed_desktop_dir());
        let source = match entry.source {
            DiscoverySource::Managed => crate::model::GameSource::ManagedDesktop,
            DiscoverySource::Manual => crate::model::GameSource::Manual,
            _ => crate::model::GameSource::DesktopEntry,
        };
        let visible_to_profiles = self.default_import_visibility();
        self.library.games.push(GameEntry {
            id: id.clone(),
            desktop_file: entry.file.path.clone(),
            title_override: None,
            desktop_name: Some(entry.file.name.clone()),
            desktop_icon: entry.file.icon.clone(),
            cover: None,
            background: None,
            visible_to_profiles,
            hidden: false,
            sort_order: max_sort + 10,
            managed,
            created_at_unix: now_unix(),
            desktop_id: Some(entry.desktop_id.clone()),
            source,
            launch_method: crate::model::LaunchMethod::GioDesktop,
        });
        self.overlay = Overlay::None;
        self.save_library();
        self.focus_on_game(&id);
        self.notify(self.tf("app-imported", &[("name", entry.file.name.clone().into())]));
        log_info(
            "import",
            format!("{} -> {}", entry.file.name, entry.file.path.display()),
        );
    }

    fn default_import_visibility(&self) -> Option<Vec<String>> {
        // A newly imported entry must never become visible to a child profile
        // automatically. Parents can grant access explicitly in the game editor.
        default_visibility_for_profiles(&self.library.profiles)
    }

    pub fn focus_on_game(&mut self, id: &str) {
        if let Some(index) = self.visible_games().iter().position(|g| g.id == id) {
            self.set_focus(index, self.visible_games().len());
        }
    }

    pub fn add_search_dir(&mut self, path: PathBuf) {
        let path = if path.is_absolute() {
            path
        } else {
            self.paths.home.join(path)
        };
        if self
            .settings
            .additional_desktop_dirs
            .iter()
            .any(|p| p == &path)
        {
            self.overlay = Overlay::None;
            self.screen = Screen::Admin;
            self.admin.view = AdminView::Discovery;
            return;
        }
        self.settings.additional_desktop_dirs.push(path);
        self.save_settings();
        self.overlay = Overlay::None;
        self.screen = Screen::Admin;
        self.admin.view = AdminView::Discovery;
        self.notify_key("admin-discovery-added");
        self.rescan_discovery();
    }

    pub fn remove_search_dir(&mut self, path: &Path) {
        self.settings
            .additional_desktop_dirs
            .retain(|candidate| candidate != path);
        self.save_settings();
        self.notify_key("admin-discovery-removed");
        self.rescan_discovery();
    }

    pub fn rescan_discovery(&mut self) {
        self.discovery_rx = None;
        self.discovery_loading = false;
        self.start_discovery();
    }

    pub fn picker_use_directory(&mut self) {
        let dir = match &self.overlay {
            Overlay::Picker(state) if state.purpose == PickerPurpose::SearchDirectory => {
                state.dir.clone()
            }
            _ => return,
        };
        self.add_search_dir(dir);
    }

    pub fn open_picker(&mut self, purpose: PickerPurpose) {
        let start = match &purpose {
            PickerPurpose::ImportDesktop => {
                // Start in the user's application folder when it exists, but
                // the picker can navigate anywhere.
                let apps = self.paths.data_home.join("applications");
                if apps.is_dir() {
                    apps
                } else {
                    self.paths.home.clone()
                }
            }
            PickerPurpose::SearchDirectory => self.paths.home.clone(),
            _ => {
                let pictures = self.paths.home.join("Pictures");
                if pictures.is_dir() {
                    pictures
                } else {
                    self.paths.home.clone()
                }
            }
        };
        let mut state = FilePickerState {
            purpose,
            dir: start,
            entries: Vec::new(),
            selected: 0,
            path_input: String::new(),
            error: None,
        };
        refresh_picker(&mut state, &self.i18n);
        state.path_input = state.dir.to_string_lossy().to_string();
        self.overlay = Overlay::Picker(state);
    }

    pub fn picker_navigate(&mut self, dir: PathBuf) {
        if let Overlay::Picker(state) = &mut self.overlay {
            if dir.is_dir() {
                state.dir = dir;
                state.selected = 0;
                state.error = None;
                refresh_picker(state, &self.i18n);
                state.path_input = state.dir.to_string_lossy().to_string();
            } else {
                state.error = Some(format!("Kein Verzeichnis: {}", dir.display()));
            }
        }
    }

    pub fn picker_parent(&mut self) {
        if let Overlay::Picker(state) = &mut self.overlay {
            if let Some(parent) = state.dir.parent().map(Path::to_path_buf) {
                state.dir = parent;
                state.selected = 0;
                refresh_picker(state, &self.i18n);
                state.path_input = state.dir.to_string_lossy().to_string();
            }
        }
    }

    pub fn picker_open_path_input(&mut self) {
        let path = if let Overlay::Picker(state) = &self.overlay {
            let entered = PathBuf::from(state.path_input.trim());
            if entered.is_relative() {
                state.dir.join(entered)
            } else {
                entered
            }
        } else {
            return;
        };
        if path.is_dir() {
            self.picker_navigate(path);
        } else if path.is_file() {
            self.picker_confirm_path(path);
        } else {
            let message = self.tf(
                "app-picker-path-not-found",
                &[("path", path.display().to_string().into())],
            );
            if let Overlay::Picker(state) = &mut self.overlay {
                state.error = Some(message);
            }
        }
    }

    pub fn picker_activate_selected(&mut self) {
        let selected = match &self.overlay {
            Overlay::Picker(state) => state.entries.get(state.selected).cloned(),
            _ => None,
        };
        if let Some(entry) = selected {
            if entry.is_dir {
                self.picker_navigate(entry.path);
            } else {
                self.picker_confirm_path(entry.path);
            }
        }
    }

    pub fn picker_confirm_path(&mut self, path: PathBuf) {
        let purpose = match &self.overlay {
            Overlay::Picker(state) => state.purpose.clone(),
            _ => return,
        };
        if purpose == PickerPurpose::ImportDesktop {
            self.load_import_file(path);
            return;
        }
        if purpose == PickerPurpose::SearchDirectory {
            if path.is_dir() {
                self.add_search_dir(path);
            } else {
                let message = self.t("app-picker-no-directory");
                if let Overlay::Picker(state) = &mut self.overlay {
                    state.error = Some(message);
                }
            }
            return;
        }
        match self.import_artwork(&purpose, &path) {
            Ok(message) => {
                self.overlay = Overlay::None;
                self.notify(message);
            }
            Err(err) => {
                if let Overlay::Picker(state) = &mut self.overlay {
                    state.error = Some(err);
                }
            }
        }
    }

    fn import_artwork(&mut self, purpose: &PickerPurpose, source: &Path) -> Result<String, String> {
        match purpose {
            PickerPurpose::Cover(game_id) => {
                let title = self
                    .library
                    .game(game_id)
                    .map(GameEntry::title)
                    .ok_or_else(|| self.t("app-game-not-found"))?;
                let relative =
                    artwork::import_image(source, &self.paths, ArtworkKind::Cover, game_id)?;
                if let Some(game) = self.library.game_mut(game_id) {
                    game.cover = Some(relative);
                }
                self.save_library();
                Ok(self.tf("app-cover-updated", &[("title", title.into())]))
            }
            PickerPurpose::Background(game_id) => {
                let title = self
                    .library
                    .game(game_id)
                    .map(GameEntry::title)
                    .ok_or_else(|| self.t("app-game-not-found"))?;
                let relative =
                    artwork::import_image(source, &self.paths, ArtworkKind::Background, game_id)?;
                if let Some(game) = self.library.game_mut(game_id) {
                    game.background = Some(relative);
                }
                self.save_library();
                Ok(self.tf("app-game-background-updated", &[("title", title.into())]))
            }
            PickerPurpose::GlobalBackground => {
                let relative = artwork::import_image(
                    source,
                    &self.paths,
                    ArtworkKind::Background,
                    "global-background",
                )?;
                self.settings.background = Some(relative);
                self.save_settings();
                Ok(self.t("app-global-background-updated"))
            }
            PickerPurpose::Avatar(profile_id) => {
                let name = self
                    .library
                    .profiles
                    .iter()
                    .find(|p| &p.id == profile_id)
                    .map(|p| p.name.clone())
                    .ok_or_else(|| self.t("app-profile-not-found"))?;
                let relative =
                    artwork::import_image(source, &self.paths, ArtworkKind::Avatar, profile_id)?;
                if let Some(profile) = self
                    .library
                    .profiles
                    .iter_mut()
                    .find(|p| &p.id == profile_id)
                {
                    profile.avatar = Some(relative);
                }
                self.save_library();
                Ok(self.tf("app-avatar-updated", &[("name", name.into())]))
            }
            PickerPurpose::SearchDirectory => unreachable!(),
            PickerPurpose::ProfileBackground(profile_id) => {
                let name = self
                    .library
                    .profiles
                    .iter()
                    .find(|p| &p.id == profile_id)
                    .map(|p| p.name.clone())
                    .ok_or_else(|| self.t("app-profile-not-found"))?;
                let relative = artwork::import_image(
                    source,
                    &self.paths,
                    ArtworkKind::Background,
                    profile_id,
                )?;
                if let Some(profile) = self
                    .library
                    .profiles
                    .iter_mut()
                    .find(|p| &p.id == profile_id)
                {
                    profile.background = Some(relative);
                }
                self.save_library();
                Ok(self.tf("app-profile-background-updated", &[("name", name.into())]))
            }
            PickerPurpose::ImportDesktop => unreachable!(),
        }
    }

    pub fn clear_game_cover(&mut self, game_id: &str) {
        if let Some(game) = self.library.game_mut(game_id) {
            game.cover = None;
            self.save_library();
            self.notify_key("app-cover-removed");
        }
    }

    pub fn clear_game_background(&mut self, game_id: &str) {
        if let Some(game) = self.library.game_mut(game_id) {
            game.background = None;
            self.save_library();
            self.notify_key("app-game-background-removed");
        }
    }

    pub fn clear_global_background(&mut self) {
        self.settings.background = None;
        self.save_settings();
        self.notify_key("app-global-background-removed");
    }

    pub fn clear_profile_background(&mut self, profile_id: &str) {
        if let Some(profile) = self
            .library
            .profiles
            .iter_mut()
            .find(|p| p.id == profile_id)
        {
            profile.background = None;
            self.save_library();
            self.notify_key("app-profile-background-removed");
        }
    }

    pub fn clear_profile_avatar(&mut self, profile_id: &str) {
        if let Some(profile) = self
            .library
            .profiles
            .iter_mut()
            .find(|p| p.id == profile_id)
        {
            profile.avatar = None;
            self.save_library();
            self.notify_key("app-avatar-removed");
        }
    }

    pub fn add_profile(&mut self) {
        let ids: Vec<String> = self.library.profiles.iter().map(|p| p.id.clone()).collect();
        let id = new_id("profile", &ids);
        let name = format!("Profil {}", self.library.profiles.len() + 1);
        self.library.profiles.push(crate::model::Profile {
            id: id.clone(),
            name,
            role: crate::model::ProfileRole::Player,
            avatar: None,
            favorites: Vec::new(),
            background: None,
            is_child: false,
            daily_limit_minutes: None,
        });
        self.admin.selected_profile = Some(id);
        self.save_library();
        self.notify_key("app-new-profile");
    }

    pub fn remove_profile(&mut self, profile_id: &str) {
        let Some(index) = self
            .library
            .profiles
            .iter()
            .position(|p| p.id == profile_id)
        else {
            return;
        };
        let removing_admin = self.library.profiles[index].is_admin();
        let admin_count = self
            .library
            .profiles
            .iter()
            .filter(|p| p.is_admin())
            .count();
        if removing_admin && admin_count <= 1 {
            self.notify_warning_key("app-last-admin-remove-blocked");
            return;
        }
        if self.secrets.has_pin(profile_id)
            && self.secrets.pins.len() <= 1
            && self.library.profiles.iter().any(|profile| profile.is_child)
        {
            self.message_key(
                "app-last-pin-title",
                "app-last-pin-message",
                MessageSeverity::Error,
            );
            return;
        }
        let was_active = self
            .library
            .active_profile_id
            .as_deref()
            .map(|id| id == profile_id)
            .unwrap_or(false);
        self.library.profiles.remove(index);
        self.secrets.remove_pin(profile_id);
        self.timekeeping.remove_profile(profile_id);
        for game in &mut self.library.games {
            if let Some(ids) = &mut game.visible_to_profiles {
                ids.retain(|id| id != profile_id);
            }
        }
        if was_active || self.library.active_profile().is_none() {
            self.library.active_profile_id = self
                .library
                .profiles
                .iter()
                .find(|p| p.is_admin())
                .or_else(|| self.library.profiles.first())
                .map(|p| p.id.clone());
            self.focus = 0;
        }
        self.admin.selected_profile = None;
        if was_active {
            // The active identity changed; require a fresh admin unlock for the
            // newly active profile.
            self.admin_unlocked = false;
            self.pending_admin_view = None;
            self.screen = Screen::Library;
        }
        self.save_library();
        self.save_secrets();
        self.save_timekeeping();
        self.notify_key("app-profile-removed");
    }

    pub fn remove_game(&mut self, game_id: &str) {
        self.library.games.retain(|g| g.id != game_id);
        if self
            .admin
            .selected_game
            .as_deref()
            .map(|id| id == game_id)
            .unwrap_or(false)
        {
            self.admin.selected_game = self.library.games.first().map(|g| g.id.clone());
        }
        let count = self.visible_games().len();
        self.set_focus(self.focus, count);
        self.save_library();
        self.notify_key("app-entry-removed");
    }

    pub fn toggle_game_hidden(&mut self, game_id: &str) {
        let hidden = if let Some(game) = self.library.game_mut(game_id) {
            game.hidden = !game.hidden;
            Some(game.hidden)
        } else {
            None
        };
        if let Some(hidden) = hidden {
            self.save_library();
            self.notify(if hidden {
                self.t("app-entry-hidden")
            } else {
                self.t("app-entry-shown")
            });
        }
    }

    pub fn move_game(&mut self, game_id: &str, up: bool) {
        let mut sorted: Vec<(usize, i64)> = self
            .library
            .games
            .iter()
            .enumerate()
            .map(|(i, g)| (i, g.sort_order))
            .collect();
        sorted.sort_by_key(|(_, order)| *order);
        let Some(pos) = sorted
            .iter()
            .position(|(i, _)| self.library.games[*i].id == game_id)
        else {
            return;
        };
        let swap_with = if up {
            pos.checked_sub(1)
        } else if pos + 1 < sorted.len() {
            Some(pos + 1)
        } else {
            None
        };
        if let Some(other) = swap_with {
            let a = sorted[pos].0;
            let b = sorted[other].0;
            let a_order = self.library.games[a].sort_order;
            let b_order = self.library.games[b].sort_order;
            self.library.games[a].sort_order = b_order;
            self.library.games[b].sort_order = a_order;
            self.save_library();
        }
    }

    // ---------------------------------------------------------------------
    // Steam / Prism managed entries
    // ---------------------------------------------------------------------

    pub fn request_steam_repair(&mut self, game_id: &str) {
        let Some(game) = self.library.game(game_id).cloned() else {
            return;
        };
        let Ok(entry) = desktop::parse_desktop_file(&game.desktop_file) else {
            self.message_key(
                "app-steam-repair-not-possible-title",
                "app-desktop-unreadable",
                MessageSeverity::Error,
            );
            return;
        };
        let Some(exec) = entry.exec.as_deref() else {
            return;
        };
        let Some(steam) = desktop::detect_steam_shortcut(exec) else {
            self.notify_warning_key("app-not-steam-shortcut");
            return;
        };
        if steam.flatpak {
            self.notify_key("app-steam-already-flatpak");
            return;
        }
        if !desktop::flatpak_available() {
            self.message_key(
                "app-flatpak-missing-title",
                "app-flatpak-missing-message",
                MessageSeverity::Error,
            );
            return;
        }
        if !desktop::flatpak_app_installed(&self.paths, "com.valvesoftware.Steam") {
            self.message_key(
                "app-steam-flatpak-missing-title",
                "app-steam-flatpak-missing-message",
                MessageSeverity::Error,
            );
            return;
        }
        self.overlay = Overlay::Confirm(ConfirmDialog {
            title: self.t("app-confirm-steam-title"),
            message: self.tf(
                "app-confirm-steam-message",
                &[("id", steam.app_id.clone().into())],
            ),
            confirm_label: self.t("app-confirm-steam-label"),
            danger: false,
            action: ConfirmAction::RepairSteam(game_id.to_owned()),
            focus: 0,
        });
    }

    pub fn apply_steam_repair(&mut self, game_id: &str) {
        let Some(game) = self.library.game(game_id).cloned() else {
            return;
        };
        let entry = match desktop::parse_desktop_file(&game.desktop_file) {
            Ok(entry) => entry,
            Err(err) => {
                self.message_dialog(
                    self.t("app-steam-repair-failed"),
                    err.to_string(),
                    MessageSeverity::Error,
                );
                return;
            }
        };
        let Some(steam) = entry
            .exec
            .as_deref()
            .and_then(desktop::detect_steam_shortcut)
        else {
            return;
        };
        if steam.flatpak {
            self.notify_key("app-no-repair");
            return;
        }
        let title = self.tf("app-steam-entry-title", &[("title", game.title().into())]);
        let managed_path = match desktop::write_managed_steam(
            &self.paths,
            &steam.app_id,
            &title,
            entry.icon.as_deref(),
        ) {
            Ok(path) => path,
            Err(err) => {
                self.message_dialog(
                    self.t("app-steam-repair-failed"),
                    err,
                    MessageSeverity::Error,
                );
                return;
            }
        };
        let ids: Vec<String> = self.library.games.iter().map(|g| g.id.clone()).collect();
        let id = new_id("game", &ids);
        let max_sort = self
            .library
            .games
            .iter()
            .map(|g| g.sort_order)
            .max()
            .unwrap_or(0);
        self.library.games.push(GameEntry {
            id: id.clone(),
            desktop_file: managed_path,
            title_override: Some(title.clone()),
            desktop_name: Some(title),
            desktop_icon: Some("com.valvesoftware.Steam".to_owned()),
            cover: game.cover.clone(),
            background: game.background.clone(),
            visible_to_profiles: game.visible_to_profiles.clone(),
            hidden: false,
            sort_order: max_sort + 10,
            managed: true,
            created_at_unix: now_unix(),
            desktop_id: None,
            source: crate::model::GameSource::Steam,
            launch_method: crate::model::LaunchMethod::GioDesktop,
        });
        self.overlay = Overlay::None;
        self.save_library();
        self.focus_on_game(&id);
        self.notify_key("app-steam-managed-created");
    }

    pub fn create_prism_entry(&mut self, instance_id: &str, title: &str) -> Result<String, String> {
        let Some(instance_dir) = desktop::find_prism_instance(&self.paths, instance_id) else {
            return Err(self.tf("app-prism-instance-missing", &[("id", instance_id.into())]));
        };
        if !desktop::flatpak_available() {
            return Err(self.t("app-prism-flatpak-missing").to_owned());
        }
        if !desktop::flatpak_app_installed(&self.paths, "org.prismlauncher.PrismLauncher") {
            return Err(self.t("app-prism-not-installed").to_owned());
        }
        let label = if title.trim().is_empty() {
            desktop::prism_instance_label(instance_id, &instance_dir)
        } else {
            title.trim().to_owned()
        };
        let managed = desktop::write_managed_prism(&self.paths, instance_id, &label, None)?;
        let ids: Vec<String> = self.library.games.iter().map(|g| g.id.clone()).collect();
        let id = new_id("game", &ids);
        let max_sort = self
            .library
            .games
            .iter()
            .map(|g| g.sort_order)
            .max()
            .unwrap_or(0);
        self.library.games.push(GameEntry {
            id: id.clone(),
            desktop_file: managed,
            title_override: Some(label.clone()),
            desktop_name: Some(label.clone()),
            desktop_icon: Some("org.prismlauncher.PrismLauncher".to_owned()),
            cover: None,
            background: None,
            visible_to_profiles: self.default_import_visibility(),
            hidden: false,
            sort_order: max_sort + 10,
            managed: true,
            created_at_unix: now_unix(),
            desktop_id: None,
            source: crate::model::GameSource::Prism,
            launch_method: crate::model::LaunchMethod::GioDesktop,
        });
        self.save_library();
        self.focus_on_game(&id);
        self.notify(self.tf("app-prism-created", &[("id", instance_id.into())]));
        Ok(id)
    }

    // ---------------------------------------------------------------------
    // Desktop mode / system power
    // ---------------------------------------------------------------------

    pub fn request_minimize(&mut self, ctx: &egui::Context) {
        if self.is_active_child() {
            self.request_parent_action(ParentAction::Minimize);
        } else {
            self.minimize_to_desktop(ctx);
        }
    }

    /// Leave the borderless fullscreen overlay and minimize to the desktop. The
    /// fullscreen exit must be processed by the compositor before minimizing,
    /// otherwise some Wayland compositors ignore the minimize request.
    pub fn minimize_to_desktop(&mut self, ctx: &egui::Context) {
        self.overlay = Overlay::None;
        if self.fullscreen {
            self.fullscreen = false;
            self.pending_minimize = true;
            ctx.send_viewport_cmd(ViewportCommand::Decorations(true));
            ctx.send_viewport_cmd(ViewportCommand::Fullscreen(false));
            self.notify_key("app-minimizing-desktop");
        } else {
            ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
            self.notify_key("app-minimized");
        }
        ctx.request_repaint();
    }

    fn finish_pending_minimize(&mut self, ctx: &egui::Context) {
        if !self.pending_minimize {
            return;
        }
        self.pending_minimize = false;
        ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
        self.notify_key("app-minimized");
        ctx.request_repaint();
    }

    fn poll_system(&mut self) {
        if let Some(outcome) = self.system.poll() {
            if outcome.success {
                log_info("power", "Systembefehl an systemd übergeben.");
            } else {
                if self.is_active_child() {
                    self.allow_close = false;
                }
                log_warn("power", &outcome.message);
                let detail = if outcome.message.trim().is_empty() {
                    self.t("power-disconnected")
                } else {
                    outcome.message.clone()
                };
                self.message_dialog(
                    self.t("power-error-title"),
                    self.tf("power-error-message", &[("detail", detail.into())]),
                    MessageSeverity::Error,
                );
            }
        }
    }

    fn tick_system(&mut self, ctx: &egui::Context) {
        if self.system.tick() {
            // systemctl accepted the job. The system is going down/rebooting;
            // closing the launcher releases the session UI in any case.
            self.should_quit = true;
        } else if self.system.is_active() {
            ctx.request_repaint_after(Duration::from_millis(120));
        }
    }

    // ---------------------------------------------------------------------
    // Launch
    // ---------------------------------------------------------------------

    pub fn launch_focused(&mut self) {
        let Some(game) = self.focused_game() else {
            if self.is_admin() {
                self.open_import();
            } else {
                self.notify_warning_key("app-library-empty");
            }
            return;
        };
        self.launch_game(&game);
    }

    pub fn launch_game(&mut self, game: &GameEntry) {
        if self.launch.is_some() || self.launch_rx.is_some() {
            self.notify_warning_key("app-launch-running");
            return;
        }
        if let Some(started) = self.last_launch_started {
            if started.elapsed() < Duration::from_millis(900) {
                return;
            }
        }
        if !self.check_start_allowed() {
            return;
        }
        if self.gio_message.is_some() {
            self.message_key(
                "app-gio-missing-title",
                "app-gio-missing-message",
                MessageSeverity::Error,
            );
            return;
        }
        let title = game.title();
        if !game.desktop_file.is_file() {
            let title = self.t("app-desktop-missing-title");
            let message = self.tf(
                "app-desktop-missing-message",
                &[("path", game.desktop_file.display().to_string().into())],
            );
            self.message_dialog(title, message, MessageSeverity::Error);
            return;
        }
        log_info(
            "launch",
            format!("gio launch {}", game.desktop_file.display()),
        );
        self.launch_rx = Some(launch::spawn_gio_launch(
            game.desktop_file.clone(),
            title.clone(),
            self.paths.clone(),
        ));
        self.launch = Some(LaunchState {
            title: title.clone(),
            started: Instant::now(),
            outcome: None,
            completed_at: None,
        });
        self.last_launch_started = Some(Instant::now());
        self.overlay = Overlay::None;
    }

    fn poll_launch(&mut self) {
        let result = self.launch_rx.as_ref().map(Receiver::try_recv);
        match result {
            Some(Ok(outcome)) => {
                self.launch_rx = None;
                if outcome.success {
                    log_info("launch", format!("{}: {}", outcome.title, outcome.message));
                } else {
                    log_warn("launch", format!("{}: {}", outcome.title, outcome.message));
                }
                if outcome.success {
                    if let Some(state) = &mut self.launch {
                        state.outcome = Some(outcome);
                        state.completed_at = Some(Instant::now());
                    } else {
                        self.notify(self.tf(
                            "app-launch-sent",
                            &[("title", outcome.title.clone().into())],
                        ));
                    }
                } else {
                    self.launch = None;
                    let title = self.tf(
                        "app-launch-failed",
                        &[("title", outcome.title.clone().into())],
                    );
                    self.message_dialog(title, outcome.message, MessageSeverity::Error);
                }
            }
            Some(Err(std::sync::mpsc::TryRecvError::Empty)) => {}
            Some(Err(std::sync::mpsc::TryRecvError::Disconnected)) => {
                self.launch_rx = None;
                self.launch = None;
                self.message_key(
                    "app-launch-thread-ended-title",
                    "app-launch-thread-ended-message",
                    MessageSeverity::Error,
                );
            }
            None => {}
        }
    }

    fn tick_launch_auto_close(&mut self) {
        let timed_out = self
            .launch
            .as_ref()
            .and_then(|state| state.completed_at)
            .map(|completed| completed.elapsed() > Duration::from_millis(2200))
            .unwrap_or(false);
        if timed_out {
            let title = self
                .launch
                .as_ref()
                .map(|state| state.title.clone())
                .unwrap_or_default();
            self.launch = None;
            self.notify(self.tf("app-launch-note", &[("title", title.into())]));
        }
    }

    // ---------------------------------------------------------------------
    // Controller / input
    // ---------------------------------------------------------------------

    fn update_window_focus(&mut self, ctx: &egui::Context) {
        let focused = ctx.input(|input| input.focused);
        let changed = match self.window_focused {
            None => {
                self.window_focused = Some(focused);
                self.controller.set_active(focused);
                true
            }
            Some(previous) if previous != focused => {
                self.window_focused = Some(focused);
                self.controller.set_active(focused);
                true
            }
            _ => false,
        };
        if changed {
            if focused {
                self.discard_controller_nav = true;
                // A previous parent-authorized minimise is over once the user
                // restores the launcher.
                self.allow_child_minimize = false;
                log_info(
                    "controller",
                    "Fensterfokus gewonnen; Eingabepuffer wird verworfen.",
                );
            } else {
                // A confirm that was queued by the controller must not fire
                // into an unfocused admin view.
                self.admin.pending_steps = 0;
                self.admin.pending_confirm = false;
                log_info(
                    "controller",
                    "Fensterfokus verloren; Gamepad-Navigation pausiert.",
                );
            }
        }
    }

    fn handle_child_window_controls(&mut self, ctx: &egui::Context) {
        // Best effort against WM-level close/minimise paths. Alt+F4 and window
        // buttons are intentionally routed through the same parent PIN as the
        // in-app actions.
        if ctx.input(|input| input.viewport().close_requested())
            && self.is_active_child()
            && !self.allow_close
        {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            // Do not replace an already open PIN/confirmation dialog when the
            // compositor keeps reporting the close request for a few frames.
            if !matches!(self.overlay, Overlay::Pin(_) | Overlay::Confirm(_)) {
                self.request_parent_action(ParentAction::Quit);
            }
        }

        let minimized = ctx
            .input(|input| input.viewport().minimized)
            .unwrap_or(false);
        if minimized && self.is_active_child() && !self.allow_child_minimize {
            ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
            self.notify_warning_key("app-minimize-parent-only");
        }
    }

    fn poll_controller(&mut self, ctx: &egui::Context) {
        let window_focused = ctx.input(|input| input.focused);
        let discard_nav = std::mem::take(&mut self.discard_controller_nav);
        for event in self.controller.drain() {
            match event {
                ControllerEvent::Ready(count) => {
                    self.controller_ready = true;
                    if count == 0 {
                        log_info("controller", "Kein Controller beim Start gefunden.");
                    } else {
                        log_info("controller", format!("{count} Controller bereit."));
                    }
                }
                ControllerEvent::Connected(name) => {
                    log_info("controller", format!("Verbunden: {name}"));
                    self.controllers.push(name.clone());
                    if window_focused {
                        self.notify(self.tf("app-controller-connected", &[("name", name.into())]));
                    }
                }
                ControllerEvent::Disconnected(name) => {
                    let message = self.tf(
                        "app-controller-disconnected",
                        &[("name", name.clone().into())],
                    );
                    log_info("controller", &message);
                    if let Some(index) = self.controllers.iter().position(|n| n == &name) {
                        self.controllers.remove(index);
                    }
                    if window_focused {
                        self.notify_warning(message);
                    }
                }
                ControllerEvent::Error(err) => {
                    log_warn("controller", &err);
                    self.controller_error = Some(err.clone());
                    if window_focused {
                        self.notify_warning(err);
                    }
                }
                ControllerEvent::Nav(action) => {
                    if accept_controller_nav(window_focused, discard_nav) {
                        self.handle_nav(ctx, action);
                    }
                }
            }
        }
    }

    fn inject_key(&self, ctx: &egui::Context, key: Key, modifiers: Modifiers) {
        ctx.input_mut(|input| {
            input.events.push(egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            });
        });
    }

    fn handle_admin_controller_nav(&mut self, _ctx: &egui::Context, action: NavAction) {
        match action {
            NavAction::Up | NavAction::Left => self.admin.pending_steps -= 1,
            NavAction::Down | NavAction::Right => self.admin.pending_steps += 1,
            NavAction::Confirm => self.admin.pending_confirm = true,
            NavAction::Back => {
                self.admin.pending_steps = 0;
                self.admin.pending_confirm = false;
                self.admin_back();
            }
            NavAction::Menu => {
                self.admin.pending_steps = 0;
                self.admin.pending_confirm = false;
                self.open_menu();
            }
            NavAction::PrevProfile | NavAction::NextProfile => {}
        }
    }

    fn flush_admin_confirm(&mut self, ctx: &egui::Context) {
        if !self.window_focused.unwrap_or(false)
            || self.screen != Screen::Admin
            || !matches!(self.overlay, Overlay::None)
            || !self.admin.pending_confirm
            || self.admin.pending_steps != 0
        {
            return;
        }
        self.admin.pending_confirm = false;
        self.inject_key(ctx, Key::Enter, Modifiers::NONE);
    }

    pub fn handle_nav(&mut self, ctx: &egui::Context, action: NavAction) {
        if self.fatal.is_some() {
            self.handle_fatal_nav(action);
            return;
        }
        if self.system.is_active() {
            return;
        }
        if self.launch.is_some() {
            if matches!(action, NavAction::Back | NavAction::Confirm) {
                self.launch = None;
            }
            return;
        }
        if !matches!(self.overlay, Overlay::None) {
            self.handle_overlay_nav(ctx, action);
            return;
        }
        match self.screen {
            Screen::Library => match action {
                NavAction::Up | NavAction::Down | NavAction::Left | NavAction::Right => {
                    let columns = self.column_count(ctx);
                    self.move_library_focus(action, columns);
                }
                NavAction::Confirm => self.launch_focused(),
                NavAction::Back => {
                    // Nothing destructive on B from the library.
                }
                NavAction::Menu => self.open_menu(),
                NavAction::PrevProfile | NavAction::NextProfile => self.open_profiles(),
            },
            Screen::Profiles => match action {
                NavAction::Left | NavAction::Up | NavAction::PrevProfile => {
                    if !self.library.profiles.is_empty() {
                        self.profile_focus = if self.profile_focus == 0 {
                            self.library.profiles.len() - 1
                        } else {
                            self.profile_focus - 1
                        };
                        self.focus_changed_at = Instant::now();
                    }
                }
                NavAction::Right | NavAction::Down | NavAction::NextProfile => {
                    if !self.library.profiles.is_empty() {
                        self.profile_focus = (self.profile_focus + 1) % self.library.profiles.len();
                        self.focus_changed_at = Instant::now();
                    }
                }
                NavAction::Confirm => self.request_activate_profile(self.profile_focus),
                NavAction::Back => {
                    self.screen = Screen::Library;
                }
                NavAction::Menu => self.open_menu(),
            },
            Screen::Admin => self.handle_admin_controller_nav(ctx, action),
        }
    }

    fn handle_fatal_nav(&mut self, action: NavAction) {
        if let Some(fatal) = &mut self.fatal {
            match action {
                NavAction::Up | NavAction::Left => fatal.focus = 0,
                NavAction::Down | NavAction::Right => fatal.focus = 1,
                NavAction::Confirm => {
                    if fatal.focus == 1 {
                        self.reset_after_fatal();
                    } else {
                        self.should_quit = true;
                    }
                }
                _ => {}
            }
        }
    }

    pub fn reset_after_fatal(&mut self) {
        match storage::reset_library_with_backup(&self.paths) {
            Ok((library, backup)) => {
                self.library = library;
                self.fatal = None;
                self.overlay = Overlay::None;
                self.focus = 0;
                self.admin_unlocked = false;
                self.screen = Screen::Library;
                self.notify(match backup {
                    Some(path) => self.tf(
                        "app-new-library-backup",
                        &[("path", path.display().to_string().into())],
                    ),
                    None => self.t("app-new-library-created"),
                });
            }
            Err(err) => {
                let message = self.tf(
                    "app-fatal-reset-failed",
                    &[("error", err.to_string().into())],
                );
                if let Some(fatal) = &mut self.fatal {
                    fatal.message = message;
                }
            }
        }
    }

    fn handle_overlay_nav(&mut self, ctx: &egui::Context, action: NavAction) {
        let overlay = std::mem::replace(&mut self.overlay, Overlay::None);
        match overlay {
            Overlay::None => {}
            Overlay::Menu(mut menu) => match action {
                NavAction::Up => {
                    if !menu.items.is_empty() {
                        menu.focus = if menu.focus == 0 {
                            menu.items.len() - 1
                        } else {
                            menu.focus - 1
                        };
                    }
                    self.overlay = Overlay::Menu(menu);
                }
                NavAction::Down => {
                    if !menu.items.is_empty() {
                        menu.focus = (menu.focus + 1) % menu.items.len();
                    }
                    self.overlay = Overlay::Menu(menu);
                }
                NavAction::Confirm => {
                    if let Some(item) = menu.items.get(menu.focus).cloned() {
                        self.menu_focus = menu.focus;
                        self.execute_menu_action(ctx, item.action);
                    } else {
                        self.overlay = Overlay::Menu(menu);
                    }
                }
                NavAction::Back | NavAction::Menu => {
                    self.overlay = Overlay::None;
                }
                _ => {
                    self.overlay = Overlay::Menu(menu);
                }
            },
            Overlay::Import(mut state) => match action {
                NavAction::Up => {
                    if state.focus_actions {
                        state.action_index = state.action_index.saturating_sub(1);
                    } else {
                        state.selected = state.selected.saturating_sub(1);
                    }
                    self.overlay = Overlay::Import(state);
                }
                NavAction::Down => {
                    if state.focus_actions {
                        state.action_index = (state.action_index + 1).min(2);
                    } else if !state.entries.is_empty() {
                        state.selected = (state.selected + 1).min(state.entries.len() - 1);
                    }
                    self.overlay = Overlay::Import(state);
                }
                NavAction::Left => {
                    state.focus_actions = false;
                    self.overlay = Overlay::Import(state);
                }
                NavAction::Right => {
                    state.focus_actions = true;
                    self.overlay = Overlay::Import(state);
                }
                NavAction::Confirm => {
                    if state.focus_actions {
                        match state.action_index {
                            0 => {
                                if let Some(entry) = state.entries.get(state.selected).cloned() {
                                    self.import_entry(&entry);
                                } else {
                                    self.overlay = Overlay::Import(state);
                                }
                            }
                            1 => {
                                self.open_import_file_picker();
                            }
                            _ => self.overlay = Overlay::None,
                        }
                    } else if let Some(entry) = state.entries.get(state.selected).cloned() {
                        self.import_entry(&entry);
                    } else {
                        self.overlay = Overlay::Import(state);
                    }
                }
                NavAction::Back => {
                    if state.focus_actions {
                        state.focus_actions = false;
                        self.overlay = Overlay::Import(state);
                    } else {
                        self.overlay = Overlay::None;
                    }
                }
                NavAction::Menu => self.overlay = Overlay::None,
                _ => self.overlay = Overlay::Import(state),
            },
            Overlay::Picker(mut state) => match action {
                NavAction::Up => {
                    state.selected = state.selected.saturating_sub(1);
                    self.overlay = Overlay::Picker(state);
                }
                NavAction::Down => {
                    if !state.entries.is_empty() {
                        state.selected = (state.selected + 1).min(state.entries.len() - 1);
                    }
                    self.overlay = Overlay::Picker(state);
                }
                NavAction::Left => {
                    self.overlay = Overlay::Picker(state);
                    self.picker_parent();
                }
                NavAction::Right => {
                    self.overlay = Overlay::Picker(state);
                    self.picker_activate_selected();
                }
                NavAction::Confirm => {
                    self.overlay = Overlay::Picker(state);
                    self.picker_activate_selected();
                }
                NavAction::Menu if state.purpose == PickerPurpose::SearchDirectory => {
                    self.overlay = Overlay::Picker(state);
                    self.picker_use_directory();
                }
                NavAction::Back | NavAction::Menu => {
                    self.overlay = Overlay::None;
                }
                _ => self.overlay = Overlay::Picker(state),
            },
            Overlay::Pin(mut pad) => match action {
                NavAction::Left => {
                    let row_start = (pad.focus / 3) * 3;
                    pad.focus = if pad.focus > row_start {
                        pad.focus - 1
                    } else {
                        pad.focus
                    };
                    self.overlay = Overlay::Pin(pad);
                }
                NavAction::Right => {
                    let row_start = (pad.focus / 3) * 3;
                    pad.focus = if pad.focus < row_start + 2 {
                        pad.focus + 1
                    } else {
                        pad.focus
                    };
                    self.overlay = Overlay::Pin(pad);
                }
                NavAction::Up => {
                    pad.focus = pad.focus.saturating_sub(3);
                    self.overlay = Overlay::Pin(pad);
                }
                NavAction::Down => {
                    pad.focus = (pad.focus + 3).min(11);
                    self.overlay = Overlay::Pin(pad);
                }
                NavAction::Confirm => {
                    let focus = pad.focus;
                    self.overlay = Overlay::Pin(pad);
                    self.pin_press(ctx, focus);
                }
                NavAction::Back => {
                    self.overlay = Overlay::None;
                }
                NavAction::Menu => {
                    self.overlay = Overlay::Pin(pad);
                }
                _ => {
                    self.overlay = Overlay::Pin(pad);
                }
            },
            Overlay::Confirm(mut dialog) => match action {
                NavAction::Left | NavAction::Right | NavAction::Up | NavAction::Down => {
                    dialog.focus = 1 - dialog.focus.min(1);
                    self.overlay = Overlay::Confirm(dialog);
                }
                NavAction::Confirm => {
                    self.apply_confirm(dialog.action);
                }
                NavAction::Back => {
                    self.overlay = Overlay::None;
                }
                _ => {
                    self.overlay = Overlay::Confirm(dialog);
                }
            },
            Overlay::TimeExpired(mut state) => match action {
                NavAction::Up | NavAction::Left => {
                    state.focus = 0;
                    self.overlay = Overlay::TimeExpired(state);
                }
                NavAction::Down | NavAction::Right => {
                    state.focus = 1;
                    self.overlay = Overlay::TimeExpired(state);
                }
                NavAction::Confirm => {
                    if state.focus == 0 {
                        self.overlay = Overlay::None;
                        self.request_extra_time();
                    } else {
                        self.overlay = Overlay::None;
                    }
                }
                NavAction::Back | NavAction::Menu => self.overlay = Overlay::None,
                _ => self.overlay = Overlay::TimeExpired(state),
            },
            Overlay::ExtraTime(mut dialog) => match action {
                NavAction::Left if dialog.custom_mode => {
                    dialog.custom_minutes = dialog.custom_minutes.saturating_sub(5).max(5);
                    self.overlay = Overlay::ExtraTime(dialog);
                }
                NavAction::Right if dialog.custom_mode => {
                    dialog.custom_minutes = (dialog.custom_minutes + 5).min(240);
                    self.overlay = Overlay::ExtraTime(dialog);
                }
                NavAction::Up if dialog.custom_mode => {
                    dialog.custom_minutes = (dialog.custom_minutes + 15).min(240);
                    self.overlay = Overlay::ExtraTime(dialog);
                }
                NavAction::Down if dialog.custom_mode => {
                    dialog.custom_minutes = dialog.custom_minutes.saturating_sub(15).max(5);
                    self.overlay = Overlay::ExtraTime(dialog);
                }
                NavAction::Up | NavAction::Left if !dialog.custom_mode => {
                    dialog.focus = if dialog.focus == 0 {
                        3
                    } else {
                        dialog.focus - 1
                    };
                    self.overlay = Overlay::ExtraTime(dialog);
                }
                NavAction::Down | NavAction::Right if !dialog.custom_mode => {
                    dialog.focus = (dialog.focus + 1).min(3);
                    self.overlay = Overlay::ExtraTime(dialog);
                }
                NavAction::Confirm => {
                    if dialog.custom_mode {
                        match self.grant_extra_minutes(&dialog.profile_id, dialog.custom_minutes) {
                            Ok(()) => {}
                            Err(err) => {
                                dialog.error = Some(err);
                                self.overlay = Overlay::ExtraTime(dialog);
                            }
                        }
                    } else {
                        match dialog.focus {
                            0 => {
                                if let Err(err) = self.grant_extra_minutes(&dialog.profile_id, 5) {
                                    dialog.error = Some(err);
                                    self.overlay = Overlay::ExtraTime(dialog);
                                }
                            }
                            1 => {
                                if let Err(err) = self.grant_extra_minutes(&dialog.profile_id, 15) {
                                    dialog.error = Some(err);
                                    self.overlay = Overlay::ExtraTime(dialog);
                                }
                            }
                            2 => {
                                if let Err(err) = self.grant_extra_minutes(&dialog.profile_id, 30) {
                                    dialog.error = Some(err);
                                    self.overlay = Overlay::ExtraTime(dialog);
                                }
                            }
                            _ => {
                                dialog.custom_mode = true;
                                dialog.custom_minutes = 15;
                                self.overlay = Overlay::ExtraTime(dialog);
                            }
                        }
                    }
                }
                NavAction::Back => {
                    if dialog.custom_mode {
                        dialog.custom_mode = false;
                        self.overlay = Overlay::ExtraTime(dialog);
                    } else {
                        self.overlay = Overlay::None;
                    }
                }
                NavAction::Menu => self.overlay = Overlay::None,
                _ => self.overlay = Overlay::ExtraTime(dialog),
            },
            Overlay::Message(_) => {
                if matches!(
                    action,
                    NavAction::Confirm | NavAction::Back | NavAction::Menu
                ) {
                    self.overlay = Overlay::None;
                }
            }
        }
    }

    pub fn pin_press(&mut self, ctx: &egui::Context, index: usize) {
        let overlay = std::mem::replace(&mut self.overlay, Overlay::None);
        let Overlay::Pin(mut pad) = overlay else {
            return;
        };
        if index > 11 {
            return;
        }
        match index {
            9 => {
                pad.digits.pop();
                pad.error = None;
            }
            11 => match pad.purpose.clone() {
                PinPurpose::Unlock { profile_id } => {
                    if crate::security::verify_pin(
                        &pad.digits,
                        self.secrets
                            .pins
                            .get(&profile_id)
                            .map(String::as_str)
                            .unwrap_or(""),
                    ) {
                        let view = self.pending_admin_view.take().unwrap_or(AdminView::Menu);
                        self.enter_admin(view);
                        self.notify_key("app-admin-unlocked");
                        return;
                    } else {
                        pad.error = Some(self.t("app-pin-wrong"));
                        pad.digits.clear();
                    }
                }
                PinPurpose::ParentAuth { action } => {
                    if self.verify_any_admin_pin(&pad.digits) {
                        self.execute_parent_action(ctx, action);
                        return;
                    } else {
                        pad.error = Some(self.t("app-admin-pin-wrong"));
                        pad.digits.clear();
                    }
                }
                PinPurpose::Set { profile_id } => {
                    if let Err(err) = crate::security::validate_pin(&pad.digits) {
                        pad.error = Some(err);
                    } else {
                        let first = pad.digits.clone();
                        pad.digits.clear();
                        pad.error = None;
                        pad.purpose = PinPurpose::ConfirmSet { profile_id, first };
                    }
                }
                PinPurpose::ConfirmSet { profile_id, first } => {
                    if pad.digits != first {
                        pad.error = Some(self.t("app-pin-mismatch"));
                        pad.digits.clear();
                    } else {
                        match self.secrets.set_pin(&profile_id, &pad.digits) {
                            Ok(()) => {
                                self.save_secrets();
                                self.overlay = Overlay::None;
                                self.notify_key("app-admin-pin-set");
                                return;
                            }
                            Err(err) => {
                                pad.error = Some(err);
                                pad.digits.clear();
                            }
                        }
                    }
                }
            },
            _ => {
                let digit = if index == 10 {
                    '0'
                } else {
                    (b'1' + index as u8) as char
                };
                if pad.digits.len() < crate::security::MAX_PIN_LEN {
                    pad.digits.push(digit);
                    pad.error = None;
                }
            }
        }
        self.overlay = Overlay::Pin(pad);
    }

    pub fn open_set_pin(&mut self) {
        let Some(profile_id) = self.active_profile_id().map(ToOwned::to_owned) else {
            return;
        };
        self.overlay = Overlay::Pin(PinPad {
            digits: String::new(),
            purpose: PinPurpose::Set { profile_id },
            focus: 0,
            error: None,
        });
    }

    pub fn request_remove_pin(&mut self) {
        let Some(profile_id) = self.active_profile_id().map(ToOwned::to_owned) else {
            return;
        };
        self.overlay = Overlay::Confirm(ConfirmDialog {
            title: self.t("app-remove-pin-title"),
            message: self.t("app-remove-pin-message"),
            confirm_label: self.t("app-remove-pin-label"),
            danger: true,
            action: ConfirmAction::RemovePin(profile_id),
            focus: 0,
        });
    }

    pub fn apply_confirm(&mut self, action: ConfirmAction) {
        match action {
            ConfirmAction::RemoveGame(id) => {
                self.remove_game(&id);
                self.overlay = Overlay::None;
                self.admin.view = AdminView::Games;
            }
            ConfirmAction::RemoveProfile(id) => {
                self.remove_profile(&id);
                self.overlay = Overlay::None;
                self.admin.view = AdminView::Profiles;
            }
            ConfirmAction::RemovePin(profile_id) => {
                if self.secrets.pins.len() <= 1
                    && self.library.profiles.iter().any(|profile| profile.is_child)
                {
                    self.overlay = Overlay::None;
                    self.admin.view = AdminView::Pin;
                    self.message_key(
                        "app-last-pin-title",
                        "app-last-pin-message",
                        MessageSeverity::Error,
                    );
                    return;
                }
                self.secrets.remove_pin(&profile_id);
                self.save_secrets();
                self.overlay = Overlay::None;
                self.admin.view = AdminView::Pin;
                self.notify_key("app-pin-removed");
            }
            ConfirmAction::RepairSteam(id) => {
                self.apply_steam_repair(&id);
                self.admin.view = AdminView::Integrations;
            }
            ConfirmAction::Reboot => {
                self.overlay = Overlay::None;
                self.allow_close = true;
                self.system.start(SystemAction::Reboot);
            }
            ConfirmAction::PowerOff => {
                self.overlay = Overlay::None;
                self.allow_close = true;
                self.system.start(SystemAction::PowerOff);
            }
            ConfirmAction::Quit => {
                self.overlay = Overlay::None;
                self.allow_close = true;
                self.should_quit = true;
            }
        }
    }

    pub fn request_remove_game(&mut self, game_id: &str) {
        let title = self
            .library
            .game(game_id)
            .map(GameEntry::title)
            .unwrap_or_else(|| self.t("app-entry"));
        self.overlay = Overlay::Confirm(ConfirmDialog {
            title: self.t("app-remove-entry-title"),
            message: self.tf(
                "app-remove-entry-message",
                &[("title", title.clone().into())],
            ),
            confirm_label: self.t("app-remove-entry-label"),
            danger: true,
            action: ConfirmAction::RemoveGame(game_id.to_owned()),
            focus: 0,
        });
    }

    pub fn request_remove_profile(&mut self, profile_id: &str) {
        let name = self
            .library
            .profiles
            .iter()
            .find(|p| p.id == profile_id)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| self.t("app-profile"));
        self.overlay = Overlay::Confirm(ConfirmDialog {
            title: self.t("app-remove-profile-title"),
            message: self.tf(
                "app-remove-profile-message",
                &[("name", name.clone().into())],
            ),
            confirm_label: self.t("app-remove-profile-label"),
            danger: true,
            action: ConfirmAction::RemoveProfile(profile_id.to_owned()),
            focus: 0,
        });
    }

    // ---------------------------------------------------------------------
    // Frame
    // ---------------------------------------------------------------------

    fn handle_global_keys(&mut self, ctx: &egui::Context) {
        let quit = ctx.input_mut(|input| input.consume_key(Modifiers::CTRL, Key::Q));
        if quit {
            if self.is_active_child() {
                self.request_parent_action(ParentAction::Quit);
            } else {
                self.should_quit = true;
            }
        }
        if !self.system.is_active() {
            let fullscreen = ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::F11));
            if fullscreen {
                self.fullscreen = !self.fullscreen;
                ctx.send_viewport_cmd(ViewportCommand::Decorations(!self.fullscreen));
                ctx.send_viewport_cmd(ViewportCommand::Fullscreen(self.fullscreen));
            }
        }
    }

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        if self.fatal.is_some() {
            let up = ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowUp));
            let down = ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowDown));
            let confirm = ctx.input_mut(|i| {
                i.consume_key(Modifiers::NONE, Key::Enter)
                    || i.consume_key(Modifiers::NONE, Key::Space)
            });
            if up {
                self.handle_nav(ctx, NavAction::Up);
            }
            if down {
                self.handle_nav(ctx, NavAction::Down);
            }
            if confirm {
                self.handle_nav(ctx, NavAction::Confirm);
            }
            return;
        }

        // During a confirmed power action, do not let controller/keyboard input
        // start games or change the underlying screen. Ctrl+Q remains available
        // through handle_global_keys.
        if self.system.is_active() {
            return;
        }

        // Escape must work even while a path text field has focus.
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            self.handle_nav(ctx, NavAction::Back);
        }

        if self.launch.is_some() {
            if ctx.input_mut(|i| {
                i.consume_key(Modifiers::NONE, Key::Enter)
                    || i.consume_key(Modifiers::NONE, Key::Space)
            }) {
                self.launch = None;
            }
            return;
        }

        if matches!(self.overlay, Overlay::Pin(_)) {
            let mut digits = Vec::new();
            for (key, digit) in [
                (Key::Num0, '0'),
                (Key::Num1, '1'),
                (Key::Num2, '2'),
                (Key::Num3, '3'),
                (Key::Num4, '4'),
                (Key::Num5, '5'),
                (Key::Num6, '6'),
                (Key::Num7, '7'),
                (Key::Num8, '8'),
                (Key::Num9, '9'),
            ] {
                if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, key)) {
                    digits.push(digit);
                }
            }
            let backspace = ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Backspace));
            let enter = ctx.input_mut(|i| {
                i.consume_key(Modifiers::NONE, Key::Enter)
                    || i.consume_key(Modifiers::NONE, Key::Space)
            });
            for digit in digits {
                if let Overlay::Pin(p) = &mut self.overlay {
                    if p.digits.len() < crate::security::MAX_PIN_LEN {
                        p.digits.push(digit);
                        p.error = None;
                    }
                }
            }
            if backspace {
                if let Overlay::Pin(p) = &mut self.overlay {
                    p.digits.pop();
                }
            }
            if enter {
                self.pin_press(ctx, 11);
            }
            return;
        }

        if ctx.wants_keyboard_input() {
            return;
        }

        if self.screen == Screen::Admin && matches!(self.overlay, Overlay::None) {
            // egui's native focus navigation handles arrow keys and Enter for
            // standard widgets. Only the menu key is handled here.
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::M)) {
                self.open_menu();
            }
            return;
        }

        let mut actions = Vec::new();
        let all = [
            (Key::ArrowUp, NavAction::Up),
            (Key::ArrowDown, NavAction::Down),
            (Key::ArrowLeft, NavAction::Left),
            (Key::ArrowRight, NavAction::Right),
            (Key::Enter, NavAction::Confirm),
            (Key::Space, NavAction::Confirm),
            (Key::M, NavAction::Menu),
            (Key::PageUp, NavAction::PrevProfile),
            (Key::PageDown, NavAction::NextProfile),
        ];
        for (key, action) in all {
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, key)) {
                actions.push(action);
            }
        }
        for action in actions {
            self.handle_nav(ctx, action);
        }
    }

    fn tick_toast(&mut self, ctx: &egui::Context) {
        if let Some(toast) = &self.toast {
            if toast.created_at.elapsed() >= toast.duration {
                self.toast = None;
            } else {
                ctx.request_repaint_after(Duration::from_millis(250));
            }
        }
        if self.launch.is_some() {
            ctx.request_repaint_after(Duration::from_millis(120));
        }
    }
}

impl eframe::App for CouchDeckApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame_count += 1;
        self.update_window_focus(ctx);
        self.handle_child_window_controls(ctx);
        self.poll_controller(ctx);
        self.flush_admin_confirm(ctx);
        self.poll_launch();
        self.poll_system();
        self.poll_discovery();
        if self.discovery_loading {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
        self.finish_pending_minimize(ctx);
        self.handle_global_keys(ctx);
        self.handle_keyboard(ctx);
        self.tick_launch_auto_close();
        self.tick_time_budget(ctx);
        self.maybe_show_time_expired();
        self.tick_system(ctx);
        crate::ui::draw(self, ctx);
        self.tick_toast(ctx);
        if self.should_quit {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.fatal.is_none() {
            let _ = storage::save_library(&self.paths, &self.library);
            let _ = storage::save_settings(&self.paths, &self.settings);
            let _ = timekeeping::save(&self.paths, &self.timekeeping);
        }
    }
}

fn accept_controller_nav(window_focused: bool, discard_buffered: bool) -> bool {
    window_focused && !discard_buffered
}

pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn refresh_picker(state: &mut FilePickerState, i18n: &I18n) {
    state.entries.clear();
    let dir = state.dir.clone();
    let filter_desktop = state.purpose == PickerPurpose::ImportDesktop;
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    if let Ok(read_dir) = std::fs::read_dir(&dir) {
        for entry in read_dir.filter_map(Result::ok) {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            if is_dir {
                dirs.push(PickerEntry {
                    name,
                    path,
                    is_dir: true,
                });
            } else {
                let extension = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                let allowed = if filter_desktop {
                    extension == "desktop"
                } else {
                    matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp")
                };
                if allowed {
                    files.push(PickerEntry {
                        name,
                        path,
                        is_dir: false,
                    });
                }
            }
        }
    } else {
        state.error = Some(i18n.format(
            "app-picker-dir-unreadable",
            &[("path", dir.display().to_string().into())],
        ));
    }
    dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    state.entries = dirs.into_iter().chain(files).collect();
    if state.selected >= state.entries.len() {
        state.selected = state.entries.len().saturating_sub(1);
    }
    state.path_input = dir.to_string_lossy().to_string();
}

#[cfg(test)]
mod tests {
    use super::{
        accept_controller_nav, admin_pin_matches, default_visibility_for_profiles,
        profile_switch_requires_parent, start_is_blocked, take_usage_seconds,
    };
    use crate::model::{AdminSecrets, Profile};
    use crate::timekeeping::TimeStatus;
    use std::time::Duration;

    #[test]
    fn controller_nav_requires_window_focus_and_no_stale_buffer() {
        assert!(accept_controller_nav(true, false));
        assert!(!accept_controller_nav(false, false));
        assert!(!accept_controller_nav(true, true));
        assert!(!accept_controller_nav(false, true));
    }

    #[test]
    fn profile_switch_from_child_requires_parent_pin() {
        assert!(profile_switch_requires_parent(
            true,
            Some("child"),
            "parent"
        ));
        assert!(!profile_switch_requires_parent(
            true,
            Some("child"),
            "child"
        ));
        assert!(!profile_switch_requires_parent(
            false,
            Some("parent"),
            "child"
        ));
        assert!(!profile_switch_requires_parent(true, None, "parent"));
    }

    #[test]
    fn fractional_time_is_not_lost_between_frames() {
        let mut remainder = Duration::ZERO;
        assert_eq!(
            take_usage_seconds(Duration::from_millis(400), &mut remainder),
            0
        );
        assert_eq!(
            take_usage_seconds(Duration::from_millis(400), &mut remainder),
            0
        );
        assert_eq!(
            take_usage_seconds(Duration::from_millis(400), &mut remainder),
            1
        );
        assert!(remainder < Duration::from_secs(1));
        assert_eq!(
            take_usage_seconds(Duration::from_millis(600), &mut remainder),
            0
        );
        assert_eq!(
            take_usage_seconds(Duration::from_millis(200), &mut remainder),
            1
        );
    }

    #[test]
    fn new_imports_are_not_visible_to_child_profiles_by_default() {
        let mut admin = Profile::new_admin("Admin");
        admin.id = "admin".to_owned();
        let mut player = Profile::new_admin("Player");
        player.id = "player".to_owned();
        player.role = crate::model::ProfileRole::Player;
        let mut child = Profile::new_admin("Child");
        child.id = "child".to_owned();
        child.role = crate::model::ProfileRole::Player;
        child.is_child = true;
        let visibility = default_visibility_for_profiles(&[admin, player, child]).unwrap();
        assert_eq!(visibility, vec!["admin".to_owned(), "player".to_owned()]);
        assert!(!visibility.iter().any(|id| id == "child"));
        assert!(default_visibility_for_profiles(&[]).is_none());
    }

    #[test]
    fn exhausted_budget_blocks_new_starts() {
        let exhausted = TimeStatus {
            limited: true,
            used_seconds: 600,
            bonus_minutes: 0,
            remaining_seconds: 0,
            remaining_minutes: 0,
            exhausted: true,
        };
        assert!(start_is_blocked(Some(&exhausted)));
        assert!(!start_is_blocked(None));
        let remaining = TimeStatus {
            remaining_seconds: 60,
            remaining_minutes: 1,
            exhausted: false,
            ..exhausted
        };
        assert!(!start_is_blocked(Some(&remaining)));
        let unlimited = TimeStatus {
            limited: false,
            exhausted: false,
            ..remaining
        };
        assert!(!start_is_blocked(Some(&unlimited)));
    }

    #[test]
    fn parent_pin_accepts_any_non_child_admin_pin() {
        let mut secrets = AdminSecrets::new();
        secrets.set_pin("admin-a", "4711").unwrap();
        secrets.set_pin("player", "1234").unwrap();
        let mut admin = Profile::new_admin("Admin A");
        admin.id = "admin-a".to_owned();
        let mut player = Profile::new_admin("Player");
        player.id = "player".to_owned();
        player.role = crate::model::ProfileRole::Player;
        let mut child_admin = Profile::new_admin("Child Admin");
        child_admin.id = "child-admin".to_owned();
        child_admin.is_child = true;
        let profiles = vec![admin, player, child_admin];
        assert!(admin_pin_matches(&profiles, &secrets, "4711"));
        assert!(!admin_pin_matches(&profiles, &secrets, "1234"));
        assert!(!admin_pin_matches(&profiles, &secrets, "0000"));
    }
}
