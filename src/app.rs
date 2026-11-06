//! Application state machine, navigation and user actions.

use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use eframe::egui;
use eframe::egui::{Key, Modifiers, ViewportCommand};

use crate::artwork::{self, ArtworkCache, ArtworkKind};
use crate::controller::{self, ControllerEvent, ControllerHandle, NavAction};
use crate::desktop::{self, DesktopFile, IconResolver};
use crate::launch::{self, LaunchOutcome};
use crate::logging::{log_error, log_info, log_warn};
use crate::model::{new_id, resolve_data_path, AdminSecrets, GameEntry, Library, Settings};
use crate::paths::XdgPaths;
use crate::storage::{self, LibraryLoad, SecretsLoad, SettingsLoad};

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
    pub entries: Vec<DesktopFile>,
    pub selected: usize,
    pub focus_actions: bool,
    pub action_index: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickerPurpose {
    Cover(String),
    Background(String),
    GlobalBackground,
    Avatar(String),
    ImportDesktop,
}

impl PickerPurpose {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Cover(_) => "Cover auswählen",
            Self::Background(_) => "Spiel-Hintergrund auswählen",
            Self::GlobalBackground => "Globales Hintergrundbild auswählen",
            Self::Avatar(_) => "Profil-Avatar auswählen",
            Self::ImportDesktop => "Desktop-Datei auswählen",
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
pub enum PinPurpose {
    Unlock { profile_id: String },
    Set { profile_id: String },
    ConfirmSet { profile_id: String, first: String },
}

pub struct PinPad {
    pub digits: String,
    pub purpose: PinPurpose,
    /// 0..=8 digits 1–9, 9 = backspace, 10 = 0, 11 = OK.
    pub focus: usize,
    pub error: Option<String>,
}

impl PinPad {
    pub fn title(&self) -> &'static str {
        match self.purpose {
            PinPurpose::Unlock { .. } => "Admin-PIN eingeben",
            PinPurpose::Set { .. } => "Neue Admin-PIN festlegen",
            PinPurpose::ConfirmSet { .. } => "Admin-PIN wiederholen",
        }
    }

    pub fn masked(&self) -> String {
        "*".repeat(self.digits.len())
    }
}

pub enum Overlay {
    None,
    Menu(MenuState),
    Import(ImportState),
    Picker(FilePickerState),
    Pin(PinPad),
    Confirm(ConfirmDialog),
    Message(MessageDialog),
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
                title: "Hinweis zum Start".to_owned(),
                message: notices.join("\n\n"),
                severity: MessageSeverity::Warning,
                close_label: "Weiter".to_owned(),
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
        app
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
        self.library.is_active_admin()
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
            close_label: "Schließen".to_owned(),
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

    pub fn activate_profile(&mut self, index: usize) {
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
        self.screen = Screen::Library;
        self.overlay = Overlay::None;
        self.save_library();
        self.notify(format!("Profil aktiv: {name}"));
    }

    pub fn open_admin(&mut self) {
        self.open_admin_view(AdminView::Menu);
    }

    pub fn open_admin_games(&mut self) {
        self.open_admin_view(AdminView::Games);
    }

    fn open_admin_view(&mut self, view: AdminView) {
        if !self.is_admin() {
            self.notify_warning("Nur ein Admin-Profil darf den Admin-Bereich öffnen.");
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
                label: "Weiter".to_owned(),
                action: MenuAction::Resume,
            });
        } else {
            items.push(MenuItem {
                label: "Zurück zur Bibliothek".to_owned(),
                action: MenuAction::Library,
            });
        }
        items.push(MenuItem {
            label: "Profil wechseln".to_owned(),
            action: MenuAction::Profiles,
        });
        if self.is_admin() {
            if self.screen != Screen::Admin {
                items.push(MenuItem {
                    label: "Admin-Bereich".to_owned(),
                    action: MenuAction::Admin,
                });
            }
            items.push(MenuItem {
                label: "Desktop-Datei importieren".to_owned(),
                action: MenuAction::Import,
            });
        }
        items.push(MenuItem {
            label: if self.fullscreen {
                "Vollbild verlassen".to_owned()
            } else {
                "Vollbild".to_owned()
            },
            action: MenuAction::ToggleFullscreen,
        });
        items.push(MenuItem {
            label: "Launcher minimieren".to_owned(),
            action: MenuAction::Minimize,
        });
        items.push(MenuItem {
            label: "Über CouchDeck".to_owned(),
            action: MenuAction::About,
        });
        items.push(MenuItem {
            label: "Beenden".to_owned(),
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
                ctx.send_viewport_cmd(ViewportCommand::Fullscreen(self.fullscreen));
                self.overlay = Overlay::None;
            }
            MenuAction::Minimize => {
                ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
                self.overlay = Overlay::None;
                self.notify("Launcher minimiert. Über die Taskleiste wiederherstellen.");
            }
            MenuAction::About => {
                let gio = self
                    .gio_message
                    .as_deref()
                    .map(|e| format!("GIO: Problem – {e}"))
                    .unwrap_or_else(|| "GIO: verfügbar".to_owned());
                self.overlay = Overlay::Message(MessageDialog {
                    title: format!("{APP_NAME} {APP_VERSION}"),
                    message: format!(
                        "Controller-first Fullscreen-Spielebibliothek für KDE Plasma/Wayland.\n\n\
                         Daten: {}\nEinstellungen: {}\nCache: {}\nLog: {}\n{gio}\n\n\
                         Start erfolgt ausschließlich über `gio launch`. CouchDeck erkennt nicht, \
                         ob ein Spiel noch läuft. Die Admin-PIN ist eine lokale UI-Sperre, keine \
                         Betriebssystem-Sicherheitsgrenze.",
                        self.paths.data_dir().display(),
                        self.paths.settings_path().display(),
                        self.paths.cache_dir().display(),
                        self.paths.log_path().display(),
                    ),
                    severity: MessageSeverity::Info,
                    close_label: "Schließen".to_owned(),
                });
            }
            MenuAction::Quit => {
                self.overlay = Overlay::Confirm(ConfirmDialog {
                    title: "CouchDeck beenden?".to_owned(),
                    message: "Ein bereits gestartetes Spiel wird nicht beendet.".to_owned(),
                    confirm_label: "Beenden".to_owned(),
                    danger: false,
                    action: ConfirmAction::Quit,
                    focus: 0,
                });
            }
        }
    }

    // ---------------------------------------------------------------------
    // Import / artwork / profiles
    // ---------------------------------------------------------------------

    pub fn open_import(&mut self) {
        if !self.is_admin() {
            self.notify_warning("Nur Admins dürfen Desktop-Dateien importieren.");
            return;
        }
        if let Some(profile_id) = self.active_profile_id() {
            if self.secrets.has_pin(profile_id) && !self.admin_unlocked {
                self.notify_warning(
                    "Admin-PIN erforderlich. Nach dem Entsperren findest du den Import unter Spiele.",
                );
                self.open_admin_games();
                return;
            }
        }
        let entries = desktop::discover_applications(&self.paths);
        self.overlay = Overlay::Import(ImportState {
            entries,
            selected: 0,
            focus_actions: false,
            action_index: 0,
            error: None,
        });
    }

    pub fn open_import_file_picker(&mut self) {
        self.open_picker(PickerPurpose::ImportDesktop);
    }

    pub fn load_import_file(&mut self, path: PathBuf) {
        match desktop::parse_desktop_file(&path) {
            Ok(entry) => {
                self.overlay = Overlay::Import(ImportState {
                    entries: vec![entry],
                    selected: 0,
                    focus_actions: false,
                    action_index: 0,
                    error: None,
                });
            }
            Err(err) => {
                self.message_dialog(
                    "Desktop-Datei ungültig",
                    format!("{err}\n\nDie Datei wurde nicht verändert."),
                    MessageSeverity::Error,
                );
            }
        }
    }

    pub fn import_entry(&mut self, entry: &DesktopFile) {
        if !self.is_admin() {
            self.notify_warning("Nur Admins dürfen Einträge importieren.");
            return;
        }
        let canonical = std::fs::canonicalize(&entry.path).unwrap_or_else(|_| entry.path.clone());
        let duplicate = self.library.games.iter().any(|game| {
            let existing = std::fs::canonicalize(&game.desktop_file)
                .unwrap_or_else(|_| game.desktop_file.clone());
            existing == canonical
        });
        if duplicate {
            self.overlay = Overlay::None;
            self.notify_warning("Dieser Desktop-Eintrag ist bereits in der Bibliothek.");
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
        let managed = entry.path.starts_with(self.paths.managed_desktop_dir());
        self.library.games.push(GameEntry {
            id: id.clone(),
            desktop_file: entry.path.clone(),
            title_override: None,
            desktop_name: Some(entry.name.clone()),
            desktop_icon: entry.icon.clone(),
            cover: None,
            background: None,
            visible_to_profiles: None,
            hidden: false,
            sort_order: max_sort + 10,
            managed,
            created_at_unix: now_unix(),
        });
        self.overlay = Overlay::None;
        self.save_library();
        self.focus_on_game(&id);
        self.notify(format!("Importiert: {}", entry.name));
        log_info(
            "import",
            format!("{} -> {}", entry.name, entry.path.display()),
        );
    }

    pub fn focus_on_game(&mut self, id: &str) {
        if let Some(index) = self.visible_games().iter().position(|g| g.id == id) {
            self.set_focus(index, self.visible_games().len());
        }
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
        refresh_picker(&mut state);
        state.path_input = state.dir.to_string_lossy().to_string();
        self.overlay = Overlay::Picker(state);
    }

    pub fn picker_navigate(&mut self, dir: PathBuf) {
        if let Overlay::Picker(state) = &mut self.overlay {
            if dir.is_dir() {
                state.dir = dir;
                state.selected = 0;
                state.error = None;
                refresh_picker(state);
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
                refresh_picker(state);
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
        } else if let Overlay::Picker(state) = &mut self.overlay {
            state.error = Some(format!("Pfad nicht gefunden: {}", path.display()));
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
                    .ok_or_else(|| "Spieleintrag nicht gefunden.".to_owned())?;
                let relative =
                    artwork::import_image(source, &self.paths, ArtworkKind::Cover, game_id)?;
                if let Some(game) = self.library.game_mut(game_id) {
                    game.cover = Some(relative);
                }
                self.save_library();
                Ok(format!("Cover für {title} aktualisiert."))
            }
            PickerPurpose::Background(game_id) => {
                let title = self
                    .library
                    .game(game_id)
                    .map(GameEntry::title)
                    .ok_or_else(|| "Spieleintrag nicht gefunden.".to_owned())?;
                let relative =
                    artwork::import_image(source, &self.paths, ArtworkKind::Background, game_id)?;
                if let Some(game) = self.library.game_mut(game_id) {
                    game.background = Some(relative);
                }
                self.save_library();
                Ok(format!("Hintergrund für {title} aktualisiert."))
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
                Ok("Globales Hintergrundbild aktualisiert.".to_owned())
            }
            PickerPurpose::Avatar(profile_id) => {
                let name = self
                    .library
                    .profiles
                    .iter()
                    .find(|p| &p.id == profile_id)
                    .map(|p| p.name.clone())
                    .ok_or_else(|| "Profil nicht gefunden.".to_owned())?;
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
                Ok(format!("Avatar für {name} aktualisiert."))
            }
            PickerPurpose::ImportDesktop => unreachable!(),
        }
    }

    pub fn clear_game_cover(&mut self, game_id: &str) {
        if let Some(game) = self.library.game_mut(game_id) {
            game.cover = None;
            self.save_library();
            self.notify("Cover-Override entfernt.");
        }
    }

    pub fn clear_game_background(&mut self, game_id: &str) {
        if let Some(game) = self.library.game_mut(game_id) {
            game.background = None;
            self.save_library();
            self.notify("Spiel-Hintergrund entfernt.");
        }
    }

    pub fn clear_global_background(&mut self) {
        self.settings.background = None;
        self.save_settings();
        self.notify("Globales Hintergrundbild entfernt.");
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
            self.notify("Avatar entfernt.");
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
        });
        self.admin.selected_profile = Some(id);
        self.save_library();
        self.notify("Neues lokales Profil angelegt. Name im Feld bearbeiten.");
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
            self.notify_warning("Das letzte Admin-Profil kann nicht entfernt werden.");
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
        self.notify("Profil entfernt.");
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
        self.notify("Eintrag aus der CouchDeck-Bibliothek entfernt. Desktop-Datei unverändert.");
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
                "Eintrag ausgeblendet."
            } else {
                "Eintrag wieder sichtbar."
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
            self.message_dialog(
                "Steam-Reparatur nicht möglich",
                "Die Desktop-Datei kann nicht gelesen werden.",
                MessageSeverity::Error,
            );
            return;
        };
        let Some(exec) = entry.exec.as_deref() else {
            return;
        };
        let Some(steam) = desktop::detect_steam_shortcut(exec) else {
            self.notify_warning("Dieser Eintrag ist kein erkannter Steam-Rungameid-Shortcut.");
            return;
        };
        if steam.flatpak {
            self.notify("Der Eintrag nutzt bereits Steam als Flatpak.");
            return;
        }
        if !desktop::flatpak_available() {
            self.message_dialog(
                "Flatpak fehlt",
                "Der Befehl `flatpak` wurde nicht gefunden. Die Reparatur würde einen funktionslosen Eintrag erzeugen.",
                MessageSeverity::Error,
            );
            return;
        }
        if !desktop::flatpak_app_installed(&self.paths, "com.valvesoftware.Steam") {
            self.message_dialog(
                "Steam-Flatpak nicht gefunden",
                "com.valvesoftware.Steam wurde nicht installiert gefunden. Die Reparatur würde einen funktionslosen Eintrag erzeugen.",
                MessageSeverity::Error,
            );
            return;
        }
        self.overlay = Overlay::Confirm(ConfirmDialog {
            title: "Steam-Flatpak-Variante erzeugen?".to_owned(),
            message: format!(
                "Die Originaldatei bleibt unverändert.\n\nNeu erzeugt wird eine separate verwaltete Datei mit:\nflatpak run com.valvesoftware.Steam steam://rungameid/{}\n\nDer neue Eintrag wird zusätzlich in die Bibliothek aufgenommen.",
                steam.app_id
            ),
            confirm_label: "Verwaltete Datei erzeugen".to_owned(),
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
                    "Steam-Reparatur fehlgeschlagen",
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
            self.notify_error("Steam-App-ID konnte nicht ermittelt werden.");
            return;
        };
        if steam.flatpak {
            self.notify("Keine Reparatur nötig.");
            return;
        }
        let title = format!("{} (Steam Flatpak)", game.title());
        let managed_path = match desktop::write_managed_steam(
            &self.paths,
            &steam.app_id,
            &title,
            entry.icon.as_deref(),
        ) {
            Ok(path) => path,
            Err(err) => {
                self.message_dialog(
                    "Steam-Reparatur fehlgeschlagen",
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
        });
        self.overlay = Overlay::None;
        self.save_library();
        self.focus_on_game(&id);
        self.notify("Verwalteter Steam-Flatpak-Eintrag erzeugt. Original unverändert.");
    }

    pub fn create_prism_entry(&mut self, instance_id: &str, title: &str) -> Result<String, String> {
        let Some(instance_dir) = desktop::find_prism_instance(&self.paths, instance_id) else {
            return Err(format!(
                "Prism-Instanzordner '{instance_id}' wurde nicht gefunden. --launch erwartet den Ordnernamen."
            ));
        };
        if !desktop::flatpak_available() {
            return Err(
                "Der Befehl `flatpak` wurde nicht gefunden. Der erzeugte Eintrag würde nicht starten."
                    .to_owned(),
            );
        }
        if !desktop::flatpak_app_installed(&self.paths, "org.prismlauncher.PrismLauncher") {
            return Err(
                "org.prismlauncher.PrismLauncher ist nicht als Flatpak installiert. Der erzeugte Eintrag würde nicht starten."
                    .to_owned(),
            );
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
            visible_to_profiles: None,
            hidden: false,
            sort_order: max_sort + 10,
            managed: true,
            created_at_unix: now_unix(),
        });
        self.save_library();
        self.focus_on_game(&id);
        self.notify(format!(
            "Prism-Eintrag für '{instance_id}' erzeugt. Cover kann jetzt zugewiesen werden."
        ));
        Ok(id)
    }

    // ---------------------------------------------------------------------
    // Launch
    // ---------------------------------------------------------------------

    pub fn launch_focused(&mut self) {
        let Some(game) = self.focused_game() else {
            if self.is_admin() {
                self.open_import();
            } else {
                self.notify_warning("Die Bibliothek ist leer. Ein Admin kann Spiele hinzufügen.");
            }
            return;
        };
        self.launch_game(&game);
    }

    pub fn launch_game(&mut self, game: &GameEntry) {
        if self.launch.is_some() || self.launch_rx.is_some() {
            self.notify_warning("Ein Startvorgang läuft bereits.");
            return;
        }
        if let Some(started) = self.last_launch_started {
            if started.elapsed() < Duration::from_millis(900) {
                return;
            }
        }
        if self.gio_message.is_some() {
            self.message_dialog(
                "GIO fehlt",
                "Ohne `gio` (GLib/GIO) können keine Desktop-Einträge gestartet werden. Bitte GLib installieren bzw. die Bazzite-Installation prüfen.",
                MessageSeverity::Error,
            );
            return;
        }
        let title = game.title();
        if !game.desktop_file.is_file() {
            self.message_dialog(
                "Desktop-Datei fehlt",
                format!(
                    "{}\n\nDer Eintrag bleibt in der Bibliothek, kann aber nicht gestartet werden. Ein Admin kann ihn entfernen oder erneut importieren.",
                    game.desktop_file.display()
                ),
                MessageSeverity::Error,
            );
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
                        self.notify(format!("Startbefehl gesendet: {}", outcome.title));
                    }
                } else {
                    self.launch = None;
                    self.message_dialog(
                        format!("Start fehlgeschlagen: {}", outcome.title),
                        outcome.message,
                        MessageSeverity::Error,
                    );
                }
            }
            Some(Err(std::sync::mpsc::TryRecvError::Empty)) => {}
            Some(Err(std::sync::mpsc::TryRecvError::Disconnected)) => {
                self.launch_rx = None;
                self.launch = None;
                self.message_dialog(
                    "Start-Thread beendet",
                    "Der Startvorgang wurde unerwartet beendet. Bitte Log prüfen.",
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
            self.notify(format!(
                "Startbefehl für {title} gesendet. CouchDeck erkennt das Spielende nicht automatisch."
            ));
        }
    }

    // ---------------------------------------------------------------------
    // Controller / input
    // ---------------------------------------------------------------------

    fn poll_controller(&mut self, ctx: &egui::Context) {
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
                    self.notify(format!("Controller verbunden: {name}"));
                }
                ControllerEvent::Disconnected(name) => {
                    let message =
                        format!("Controller getrennt: {name}. Andere Eingabegeräte bleiben aktiv.");
                    log_info("controller", &message);
                    if let Some(index) = self.controllers.iter().position(|n| n == &name) {
                        self.controllers.remove(index);
                    }
                    self.notify_warning(message);
                }
                ControllerEvent::Error(err) => {
                    log_warn("controller", &err);
                    self.controller_error = Some(err.clone());
                    self.notify_warning(err);
                }
                ControllerEvent::Nav(action) => self.handle_nav(ctx, action),
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
        if self.screen != Screen::Admin
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
                NavAction::Confirm => self.activate_profile(self.profile_focus),
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
                    Some(path) => format!(
                        "Neue Bibliothek angelegt. Alte Datei gesichert unter {}.",
                        path.display()
                    ),
                    None => "Neue Bibliothek angelegt.".to_owned(),
                });
            }
            Err(err) => {
                if let Some(fatal) = &mut self.fatal {
                    fatal.message = format!("Zurücksetzen fehlgeschlagen: {err}");
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
                    self.pin_press(focus);
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

    pub fn pin_press(&mut self, index: usize) {
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
                        self.notify("Admin-Bereich entsperrt.");
                        return;
                    } else {
                        pad.error = Some("PIN falsch.".to_owned());
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
                        pad.error = Some("Die PINs stimmen nicht überein.".to_owned());
                        pad.digits.clear();
                    } else {
                        match self.secrets.set_pin(&profile_id, &pad.digits) {
                            Ok(()) => {
                                self.save_secrets();
                                self.overlay = Overlay::None;
                                self.notify("Admin-PIN gesetzt (Argon2id, gehasht).");
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
            title: "Admin-PIN entfernen?".to_owned(),
            message: "Danach ist der Admin-Bereich ohne PIN erreichbar. Das ist nur eine lokale UI-Sperre."
                .to_owned(),
            confirm_label: "PIN entfernen".to_owned(),
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
                self.secrets.remove_pin(&profile_id);
                self.save_secrets();
                self.overlay = Overlay::None;
                self.admin.view = AdminView::Pin;
                self.notify("Admin-PIN entfernt.");
            }
            ConfirmAction::RepairSteam(id) => {
                self.apply_steam_repair(&id);
                self.admin.view = AdminView::Integrations;
            }
            ConfirmAction::Quit => {
                self.overlay = Overlay::None;
                self.should_quit = true;
            }
        }
    }

    pub fn request_remove_game(&mut self, game_id: &str) {
        let title = self
            .library
            .game(game_id)
            .map(GameEntry::title)
            .unwrap_or_else(|| "Eintrag".to_owned());
        self.overlay = Overlay::Confirm(ConfirmDialog {
            title: "Eintrag entfernen?".to_owned(),
            message: format!(
                "'{title}' wird nur aus der CouchDeck-Bibliothek entfernt. Die Desktop-Datei und das Spiel bleiben unverändert."
            ),
            confirm_label: "Aus Bibliothek entfernen".to_owned(),
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
            .unwrap_or_else(|| "Profil".to_owned());
        self.overlay = Overlay::Confirm(ConfirmDialog {
            title: "Profil entfernen?".to_owned(),
            message: format!(
                "'{name}' und die zugehörige PIN werden aus der lokalen CouchDeck-Datei entfernt. Linux-Konten und Spiele bleiben unverändert."
            ),
            confirm_label: "Profil entfernen".to_owned(),
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
            self.should_quit = true;
        }
        let fullscreen = ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::F11));
        if fullscreen {
            self.fullscreen = !self.fullscreen;
            ctx.send_viewport_cmd(ViewportCommand::Fullscreen(self.fullscreen));
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
                self.pin_press(11);
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
        self.poll_controller(ctx);
        self.flush_admin_confirm(ctx);
        self.poll_launch();
        self.handle_global_keys(ctx);
        self.handle_keyboard(ctx);
        self.tick_launch_auto_close();
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
        }
    }
}

pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn refresh_picker(state: &mut FilePickerState) {
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
        state.error = Some(format!("Verzeichnis nicht lesbar: {}", dir.display()));
    }
    dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    state.entries = dirs.into_iter().chain(files).collect();
    if state.selected >= state.entries.len() {
        state.selected = state.entries.len().saturating_sub(1);
    }
    state.path_input = dir.to_string_lossy().to_string();
}
