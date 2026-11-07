//! `.desktop` file parsing, XDG discovery, icon lookup and integration
//! diagnostics.
//!
//! # Launching rule
//! CouchDeck **never** interprets `Exec=` itself. The raw value is displayed
//! to the administrator for review, but actual launches always go through
//! `gio launch /absolute/path.desktop` (see [`crate::launch`]). The small
//! tokenizer below exists only to produce read-only diagnostics and to build
//! explicitly managed desktop files.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::env;
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::paths::XdgPaths;

#[derive(Debug, Clone)]
pub struct DesktopFile {
    pub path: PathBuf,
    pub name: String,
    pub generic_name: Option<String>,
    pub comment: Option<String>,
    pub icon: Option<String>,
    /// Raw `Exec=` value, exactly as found in the file (minus surrounding
    /// whitespace). Never passed to a shell.
    pub exec: Option<String>,
    pub try_exec: Option<String>,
    pub workdir: Option<String>,
    pub categories: Vec<String>,
    pub mime_types: Vec<String>,
    pub terminal: bool,
    pub dbus_activatable: bool,
    pub no_display: bool,
    pub hidden: bool,
    pub only_show_in: Vec<String>,
    pub not_show_in: Vec<String>,
    pub desktop_type: Option<String>,
    pub warnings: Vec<String>,
}

impl DesktopFile {
    pub fn file_name(&self) -> String {
        self.path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unbekannt.desktop")
            .to_owned()
    }

    pub fn executable_for_gio(&self) -> bool {
        self.dbus_activatable
            || self
                .exec
                .as_deref()
                .map(|e| !e.trim().is_empty())
                .unwrap_or(false)
    }
}

#[derive(Debug, Clone)]
pub struct DesktopError(pub String);

impl fmt::Display for DesktopError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for DesktopError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
}

impl Diagnostic {
    fn info(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Info,
            message: message.into(),
        }
    }
    fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
        }
    }
    fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
        }
    }
}

fn locale_candidates() -> Vec<String> {
    let raw = ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .find_map(|key| env::var(key).ok().filter(|s| !s.is_empty()))
        .unwrap_or_default();
    locale_candidates_from(&raw)
}

fn locale_candidates_from(raw: &str) -> Vec<String> {
    let without_encoding = raw.split('.').next().unwrap_or(raw);
    let without_modifier = without_encoding
        .split('@')
        .next()
        .unwrap_or(without_encoding);
    let mut out = Vec::new();
    if without_modifier.is_empty() || without_modifier == "C" || without_modifier == "POSIX" {
        return out;
    }
    if without_modifier.contains('_') {
        out.push(without_modifier.to_owned());
    }
    let lang = without_modifier
        .split('_')
        .next()
        .unwrap_or(without_modifier);
    if !lang.is_empty() && !out.iter().any(|s| s == lang) {
        out.push(lang.to_owned());
    }
    out
}

fn localized_value(
    entries: &BTreeMap<String, String>,
    base: &str,
    locales: &[String],
) -> Option<String> {
    for locale in locales {
        if let Some(value) = entries.get(&format!("{base}[{locale}]")) {
            if !value.trim().is_empty() {
                return Some(value.clone());
            }
        }
    }
    entries.get(base).cloned()
}

fn unescape_desktop_string(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('s') => out.push(' '),
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('\\') => out.push('\\'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

/// Escape a value for a desktop-file string key (Name/Comment).
pub fn escape_desktop_value(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\\' => out.push_str("\\\\"),
            other => out.push(other),
        }
    }
    out
}

fn parse_bool(entries: &BTreeMap<String, String>, key: &str, warnings: &mut Vec<String>) -> bool {
    match entries.get(key).map(|s| s.trim().to_ascii_lowercase()) {
        None => false,
        Some(value) if value == "true" => true,
        Some(value) if value == "false" => false,
        Some(value) => {
            warnings.push(format!(
                "{key}={value} ist kein gültiger boolescher Wert (true/false)."
            ));
            false
        }
    }
}

fn parse_list(entries: &BTreeMap<String, String>, key: &str) -> Vec<String> {
    entries
        .get(key)
        .map(|value| {
            value
                .split(';')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn read_entry_group(text: &str) -> Result<BTreeMap<String, String>, DesktopError> {
    let mut entries = BTreeMap::new();
    let mut in_group = false;
    let mut saw_group = false;
    for raw_line in text.lines() {
        let line = raw_line.trim_start_matches('\u{feff}').trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            saw_group = true;
            in_group = &line[1..line.len() - 1] == "Desktop Entry";
            continue;
        }
        if !in_group {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            entries.insert(key.trim().to_owned(), value.trim().to_owned());
        }
    }
    if !saw_group {
        return Err(DesktopError(
            "Keine [Desktop Entry]-Gruppe gefunden.".to_owned(),
        ));
    }
    if entries.is_empty() {
        return Err(DesktopError(
            "Die [Desktop Entry]-Gruppe ist leer.".to_owned(),
        ));
    }
    Ok(entries)
}

/// Parse a desktop file without executing anything.
pub fn parse_desktop_file(path: &Path) -> Result<DesktopFile, DesktopError> {
    let text = fs::read_to_string(path).map_err(|err| {
        DesktopError(format!(
            "Desktop-Datei {} kann nicht gelesen werden: {err}",
            path.display()
        ))
    })?;
    parse_desktop_text(path, &text)
}

pub fn parse_desktop_text(path: &Path, text: &str) -> Result<DesktopFile, DesktopError> {
    let locales = locale_candidates();
    parse_desktop_text_with_locales(path, text, &locales)
}

fn parse_desktop_text_with_locales(
    path: &Path,
    text: &str,
    locales: &[String],
) -> Result<DesktopFile, DesktopError> {
    let entries = read_entry_group(text)?;
    let mut warnings = Vec::new();

    let desktop_type = entries.get("Type").map(|s| s.trim().to_owned());
    if desktop_type
        .as_deref()
        .map(|t| t != "Application")
        .unwrap_or(false)
    {
        return Err(DesktopError(format!(
            "Type={} wird nicht als Anwendung unterstützt.",
            desktop_type.as_deref().unwrap_or("")
        )));
    }
    if desktop_type.is_none() {
        warnings.push("Type= fehlt; die Datei wird tolerant als Anwendung behandelt.".into());
    }

    let name = localized_value(&entries, "Name", locales)
        .map(|s| unescape_desktop_string(&s))
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unbenannt")
                .to_owned()
        });

    let exec = entries
        .get("Exec")
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty());
    let dbus_activatable = parse_bool(&entries, "DBusActivatable", &mut warnings);
    if exec.is_none() && !dbus_activatable {
        warnings.push(
            "Weder Exec= noch DBusActivatable=true vorhanden; Start kann fehlschlagen.".into(),
        );
    }

    let try_exec = entries
        .get("TryExec")
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty());

    Ok(DesktopFile {
        path: path.to_path_buf(),
        name,
        generic_name: localized_value(&entries, "GenericName", locales)
            .map(|s| unescape_desktop_string(&s)),
        comment: localized_value(&entries, "Comment", locales).map(|s| unescape_desktop_string(&s)),
        icon: entries
            .get("Icon")
            .map(|s| unescape_desktop_string(s.trim()))
            .filter(|s| !s.is_empty()),
        exec,
        try_exec,
        workdir: entries
            .get("Path")
            .map(|s| unescape_desktop_string(s.trim()))
            .filter(|s| !s.is_empty()),
        categories: parse_list(&entries, "Categories"),
        mime_types: parse_list(&entries, "MimeType"),
        terminal: parse_bool(&entries, "Terminal", &mut warnings),
        dbus_activatable,
        no_display: parse_bool(&entries, "NoDisplay", &mut warnings),
        hidden: parse_bool(&entries, "Hidden", &mut warnings),
        only_show_in: parse_list(&entries, "OnlyShowIn"),
        not_show_in: parse_list(&entries, "NotShowIn"),
        desktop_type,
        warnings,
    })
}

/// The binary that should be referenced by an autostart entry. AppImage runs
/// set `$APPIMAGE` to the stable AppImage path; using `current_exe()` there
/// would point into the temporary mount.
pub fn current_launcher_binary() -> Result<PathBuf, String> {
    if let Some(appimage) = env::var_os("APPIMAGE").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(appimage));
    }
    env::current_exe().map_err(|err| format!("Eigener Programmpfad nicht ermittelbar: {err}"))
}

/// Check executables for `TryExec` and diagnostics.
pub fn command_available(command: &str) -> bool {
    let command = command.trim();
    if command.is_empty() {
        return false;
    }
    let path = Path::new(command);
    if path.is_absolute() || command.contains('/') {
        return is_executable(path);
    }
    env::var_os("PATH")
        .map(|paths| {
            env::split_paths(&paths)
                .map(|dir| dir.join(command))
                .any(|candidate| is_executable(&candidate))
        })
        .unwrap_or(false)
}

fn is_executable(path: &Path) -> bool {
    fs::metadata(path)
        .map(|meta| meta.is_file())
        .unwrap_or(false)
        && {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::metadata(path)
                    .map(|meta| meta.permissions().mode() & 0o111 != 0)
                    .unwrap_or(false)
            }
            #[cfg(not(unix))]
            {
                true
            }
        }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoverySource {
    UserXdg,
    FlatpakUser,
    SystemXdg,
    FlatpakSystem,
    DesktopFolder,
    Managed,
    Custom,
    Manual,
}

impl DiscoverySource {
    pub fn label_key(self) -> &'static str {
        match self {
            Self::UserXdg => "discovery-source-user-xdg",
            Self::FlatpakUser => "discovery-source-flatpak-user",
            Self::SystemXdg => "discovery-source-system-xdg",
            Self::FlatpakSystem => "discovery-source-flatpak-system",
            Self::DesktopFolder => "discovery-source-desktop-folder",
            Self::Managed => "discovery-source-managed",
            Self::Custom => "discovery-source-custom",
            Self::Manual => "discovery-source-manual",
        }
    }

    pub fn is_flatpak(self) -> bool {
        matches!(self, Self::FlatpakUser | Self::FlatpakSystem)
    }

    fn priority(self) -> u8 {
        match self {
            Self::UserXdg => 0,
            Self::FlatpakUser => 1,
            Self::SystemXdg => 2,
            Self::FlatpakSystem => 3,
            Self::DesktopFolder => 4,
            Self::Custom => 5,
            Self::Managed => 6,
            Self::Manual => 7,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiscoveryRoot {
    pub path: PathBuf,
    pub source: DiscoverySource,
    pub automatic: bool,
}

#[derive(Debug, Clone)]
pub struct DiscoveryWarning {
    pub path: PathBuf,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct DiscoveredEntry {
    pub file: DesktopFile,
    pub desktop_id: String,
    pub source: DiscoverySource,
    pub source_dir: PathBuf,
    /// True when the entry would normally appear in the current desktop menu.
    /// `NoDisplay`/`Hidden`/`OnlyShowIn`/`NotShowIn` and missing `TryExec` make
    /// this false, but the entry remains visible to the administrator for a
    /// deliberate manual import.
    pub menu_visible: bool,
}

#[derive(Debug, Clone)]
pub struct DiscoveryResult {
    pub entries: Vec<DiscoveredEntry>,
    pub roots: Vec<DiscoveryRoot>,
    pub warnings: Vec<DiscoveryWarning>,
}

pub fn current_desktops() -> BTreeSet<String> {
    crate::session::current_desktop_tokens()
        .into_iter()
        .collect()
}

pub fn shown_in_desktops(entry: &DesktopFile, desktops: &BTreeSet<String>) -> bool {
    if entry
        .not_show_in
        .iter()
        .any(|d| desktops.contains(&d.to_ascii_uppercase()))
    {
        return false;
    }
    if !entry.only_show_in.is_empty()
        && !entry
            .only_show_in
            .iter()
            .any(|d| desktops.contains(&d.to_ascii_uppercase()))
    {
        return false;
    }
    true
}

fn shown_in_current_desktop(entry: &DesktopFile) -> bool {
    shown_in_desktops(entry, &current_desktops())
}

fn classify_data_dir(paths: &XdgPaths, applications: &Path) -> DiscoverySource {
    let data_home_flatpak = paths.data_home.join("flatpak/exports/share/applications");
    if applications == data_home_flatpak {
        DiscoverySource::FlatpakUser
    } else if applications.starts_with("/var/lib/flatpak/exports/share/applications") {
        DiscoverySource::FlatpakSystem
    } else if applications.starts_with(&paths.data_home) {
        DiscoverySource::UserXdg
    } else {
        DiscoverySource::SystemXdg
    }
}

/// All roots in discovery order: user XDG, Flatpak user/system, XDG data dirs,
/// the real XDG desktop folder, administrator paths and CouchDeck's managed
/// files. Discovery de-duplicates roots and desktop IDs.
pub fn discovery_roots(paths: &XdgPaths, additional: &[PathBuf]) -> Vec<DiscoveryRoot> {
    let mut roots = Vec::new();
    let mut seen = BTreeSet::new();
    let mut add = |path: PathBuf, source: DiscoverySource, automatic: bool| {
        let key = path.to_string_lossy().to_string();
        if seen.insert(key) {
            roots.push(DiscoveryRoot {
                path,
                source,
                automatic,
            });
        }
    };

    add(
        paths.data_home.join("applications"),
        DiscoverySource::UserXdg,
        true,
    );
    let flatpak_dirs = paths.flatpak_application_dirs();
    if let Some(user_flatpak) = flatpak_dirs.first() {
        add(user_flatpak.clone(), DiscoverySource::FlatpakUser, true);
    }
    for base in &paths.data_dirs {
        let applications = base.join("applications");
        if flatpak_dirs.iter().any(|dir| dir == &applications) {
            continue;
        }
        add(
            applications.clone(),
            classify_data_dir(paths, &applications),
            true,
        );
    }
    if let Some(system_flatpak) = flatpak_dirs.get(1) {
        add(system_flatpak.clone(), DiscoverySource::FlatpakSystem, true);
    }
    if let Some(desktop) = paths.desktop_dir() {
        add(desktop, DiscoverySource::DesktopFolder, true);
    }
    for additional_root in additional {
        let resolved = if additional_root.is_absolute() {
            additional_root.clone()
        } else {
            paths.home.join(additional_root)
        };
        add(resolved, DiscoverySource::Custom, false);
    }
    add(paths.managed_desktop_dir(), DiscoverySource::Managed, true);
    roots
}

fn collect_desktop_files(
    root: &DiscoveryRoot,
    dir: &Path,
    depth: usize,
    by_id: &mut BTreeMap<String, (u8, PathBuf, DiscoverySource, PathBuf)>,
    warnings: &mut Vec<DiscoveryWarning>,
) {
    if depth > 6 {
        return;
    }
    let read_dir = match fs::read_dir(dir) {
        Ok(read_dir) => read_dir,
        Err(err) => {
            if dir.exists() {
                warnings.push(DiscoveryWarning {
                    path: dir.to_path_buf(),
                    message: format!("Verzeichnis nicht lesbar: {err}"),
                });
            }
            return;
        }
    };
    for entry in read_dir.filter_map(Result::ok) {
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        // Do not follow symlinked directories; this avoids loops while still
        // allowing symlinked .desktop files to be imported.
        if file_type.is_dir() {
            collect_desktop_files(root, &path, depth + 1, by_id, warnings);
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
            continue;
        }
        let Ok(relative) = path.strip_prefix(&root.path) else {
            continue;
        };
        let desktop_id = relative
            .components()
            .filter_map(|component| match component {
                Component::Normal(part) => part.to_str(),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("-");
        if desktop_id.is_empty() {
            continue;
        }
        let priority = root.source.priority();
        by_id
            .entry(desktop_id)
            .or_insert((priority, path, root.source, root.path.clone()));
    }
}

/// Discover all desktop files, including `NoDisplay` entries that an
/// administrator may import deliberately. Use `menu_visible` on each result to
/// distinguish normal desktop-menu visibility.
pub fn discover_entries(paths: &XdgPaths, additional: &[PathBuf]) -> DiscoveryResult {
    let roots = discovery_roots(paths, additional);
    let mut by_id = BTreeMap::new();
    let mut warnings = Vec::new();
    for root in &roots {
        collect_desktop_files(root, &root.path, 0, &mut by_id, &mut warnings);
    }

    let mut entries = Vec::new();
    for (desktop_id, (_, path, source, source_dir)) in by_id {
        match parse_desktop_file(&path) {
            Ok(file) => {
                let menu_visible = !file.hidden
                    && !file.no_display
                    && shown_in_current_desktop(&file)
                    && file
                        .try_exec
                        .as_deref()
                        .map(command_available)
                        .unwrap_or(true);
                entries.push(DiscoveredEntry {
                    file,
                    desktop_id,
                    source,
                    source_dir,
                    menu_visible,
                });
            }
            Err(err) => warnings.push(DiscoveryWarning {
                path,
                message: err.to_string(),
            }),
        }
    }
    entries.sort_by(|a, b| {
        a.source
            .priority()
            .cmp(&b.source.priority())
            .then_with(|| a.file.name.to_lowercase().cmp(&b.file.name.to_lowercase()))
            .then_with(|| a.file.path.cmp(&b.file.path))
    });
    DiscoveryResult {
        entries,
        roots,
        warnings,
    }
}

/// Backwards-compatible discovery used by the normal menu-oriented code path.
/// Only entries that would appear in the current desktop menu are returned.
pub fn discover_applications(paths: &XdgPaths) -> Vec<DesktopFile> {
    discover_entries(paths, &[])
        .entries
        .into_iter()
        .filter(|entry| entry.menu_visible)
        .map(|entry| entry.file)
        .collect()
}

/// Resolve `Icon=` names against the XDG icon/pixmap locations.
///
/// SVG-only icons are intentionally not returned because the bundled image
/// decoder cannot rasterise SVG; callers fall back to a neutral tile.
pub struct IconResolver {
    index: HashMap<String, PathBuf>,
    built: bool,
    roots: Vec<PathBuf>,
}

impl IconResolver {
    pub fn new(paths: &XdgPaths) -> Self {
        Self {
            index: HashMap::new(),
            built: false,
            roots: paths.icon_search_dirs(),
        }
    }

    pub fn resolve(&mut self, icon: &str) -> Option<PathBuf> {
        let icon = icon.trim();
        if icon.is_empty() {
            return None;
        }
        let path = Path::new(icon);
        if path.is_absolute() {
            return path.is_file().then(|| path.to_path_buf());
        }
        if !self.built {
            self.build_index();
            self.built = true;
        }
        let key = icon.to_ascii_lowercase();
        if let Some(found) = self.index.get(&key) {
            return Some(found.clone());
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(icon)
            .to_ascii_lowercase();
        self.index.get(&stem).cloned()
    }

    fn build_index(&mut self) {
        let roots = self.roots.clone();
        for root in &roots {
            self.scan_icons(root, 0);
        }
    }

    fn scan_icons(&mut self, dir: &Path, depth: usize) {
        if depth > 8 {
            return;
        }
        let Ok(read_dir) = fs::read_dir(dir) else {
            return;
        };
        for entry in read_dir.filter_map(Result::ok) {
            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                self.scan_icons(&path, depth + 1);
            } else if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if matches!(ext.as_str(), "png" | "webp" | "jpg" | "jpeg") {
                    let key = stem.to_ascii_lowercase();
                    // First hit wins (user paths are scanned before system paths).
                    self.index.entry(key).or_insert(path);
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteamShortcut {
    pub app_id: String,
    pub flatpak: bool,
    pub launcher: Option<String>,
}

/// Read-only detection of a Steam URI shortcut. This never executes or
/// rewrites anything.
pub fn detect_steam_shortcut(exec: &str) -> Option<SteamShortcut> {
    let marker = "steam://rungameid/";
    let start = exec.find(marker)? + marker.len();
    let rest = &exec[start..];
    let id: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if id.is_empty() || id.len() > 10 || id.parse::<u32>().ok().filter(|n| *n > 0).is_none() {
        return None;
    }
    let tokens = diagnostic_tokens(exec);
    let flatpak = tokens.windows(3).any(|w| {
        w[0].eq_ignore_ascii_case("flatpak")
            && w[1].eq_ignore_ascii_case("run")
            && w[2].eq_ignore_ascii_case("com.valvesoftware.Steam")
    });
    let launcher = tokens.first().cloned();
    Some(SteamShortcut {
        app_id: id,
        flatpak,
        launcher,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrismLaunch {
    pub instance_id: String,
    pub flatpak: bool,
}

/// Read-only detection of a Prism `--launch <instance folder>` command.
pub fn detect_prism_launch(exec: &str) -> Option<PrismLaunch> {
    let tokens = diagnostic_tokens(exec);
    let launch_at = tokens.iter().position(|t| t == "--launch")?;
    let instance_id = tokens.get(launch_at + 1)?.trim().to_owned();
    if instance_id.is_empty()
        || instance_id.contains('/')
        || instance_id.contains('\\')
        || instance_id == "."
        || instance_id == ".."
        || instance_id.contains('\n')
    {
        return None;
    }
    let flatpak = tokens.windows(3).any(|w| {
        w[0].eq_ignore_ascii_case("flatpak")
            && w[1].eq_ignore_ascii_case("run")
            && w[2].eq_ignore_ascii_case("org.prismlauncher.PrismLauncher")
    });
    Some(PrismLaunch {
        instance_id,
        flatpak,
    })
}

/// Minimal desktop-Exec tokenizer used **only** for diagnostics. It honors
/// double quotes and backslashes but performs no variable expansion or shell
/// interpretation.
pub fn diagnostic_tokens(exec: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut escaped = false;
    for ch in exec.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        match ch {
            '\\' if quoted => escaped = true,
            '\\' => escaped = true,
            '"' => quoted = !quoted,
            c if c.is_whitespace() && !quoted => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

pub fn flatpak_app_installed(paths: &XdgPaths, app_id: &str) -> bool {
    let candidates = [
        paths.home.join(".local/share/flatpak/app").join(app_id),
        PathBuf::from("/var/lib/flatpak/app").join(app_id),
    ];
    candidates.iter().any(|path| path.is_dir())
}

pub fn flatpak_available() -> bool {
    command_available("flatpak")
}

pub fn gio_available() -> bool {
    command_available("gio")
}

pub fn prism_instance_dirs(paths: &XdgPaths) -> Vec<PathBuf> {
    vec![
        paths
            .home
            .join(".var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher/instances"),
        paths.home.join(".local/share/PrismLauncher/instances"),
        paths
            .home
            .join(".var/app/org.prismlauncher.PrismLauncher/.local/share/PrismLauncher/instances"),
    ]
}

pub fn find_prism_instance(paths: &XdgPaths, instance_id: &str) -> Option<PathBuf> {
    if instance_id.trim().is_empty()
        || instance_id.contains('/')
        || instance_id.contains('\\')
        || instance_id == "."
        || instance_id == ".."
        || instance_id.contains('\n')
    {
        return None;
    }
    prism_instance_dirs(paths)
        .into_iter()
        .map(|dir| dir.join(instance_id))
        .find(|path| path.is_dir())
}

pub fn list_prism_instances(paths: &XdgPaths, limit: usize) -> Vec<(String, PathBuf)> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for dir in prism_instance_dirs(paths) {
        let Ok(read_dir) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in read_dir.filter_map(Result::ok) {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(id) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            if id.starts_with('.') {
                continue;
            }
            if seen.insert(id.to_owned()) {
                out.push((id.to_owned(), path));
                if out.len() >= limit {
                    return out;
                }
            }
        }
    }
    out.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    out
}

fn prism_display_name(instance_dir: &Path) -> Option<String> {
    let cfg = fs::read_to_string(instance_dir.join("instance.cfg")).ok()?;
    cfg.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        if key.trim().eq_ignore_ascii_case("name") && !value.trim().is_empty() {
            Some(value.trim().to_owned())
        } else {
            None
        }
    })
}

pub fn prism_instance_label(instance_id: &str, instance_dir: &Path) -> String {
    match prism_display_name(instance_dir) {
        Some(name) if name != instance_id => format!("{name}  ({instance_id})"),
        _ => instance_id.to_owned(),
    }
}

/// Diagnostics for the import/admin screens.
pub fn analyze_entry(entry: &DesktopFile, paths: &XdgPaths) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    if !entry.path.is_file() {
        out.push(Diagnostic::error(format!(
            "Desktop-Datei fehlt: {}",
            entry.path.display()
        )));
    }
    if let Some(try_exec) = &entry.try_exec {
        if !command_available(try_exec) {
            out.push(Diagnostic::warning(format!(
                "TryExec={try_exec} wurde nicht gefunden. GIO startet den Eintrag möglicherweise nicht."
            )));
        }
    }
    if let Some(exec) = &entry.exec {
        if let Some(steam) = detect_steam_shortcut(exec) {
            if steam.flatpak {
                if !flatpak_app_installed(paths, "com.valvesoftware.Steam") {
                    out.push(Diagnostic::error(
                        "Der Eintrag nutzt Steam als Flatpak, aber com.valvesoftware.Steam wurde nicht gefunden.",
                    ));
                }
                out.push(Diagnostic::info(format!(
                    "Steam-Flatpak-Spiel mit App-ID {}.",
                    steam.app_id
                )));
            } else {
                out.push(Diagnostic::warning(format!(
                    "Steam-Spiel-URI mit App-ID {} erkannt (kein Flatpak-Aufruf). Bei Flatpak-Steam kann dieser Shortcut eventuell nicht funktionieren. CouchDeck kann nach Zustimmung eine separate verwaltete Datei erzeugen.",
                    steam.app_id
                )));
                if !flatpak_app_installed(paths, "com.valvesoftware.Steam") {
                    out.push(Diagnostic::info(
                        "com.valvesoftware.Steam ist auf diesem System nicht installiert.",
                    ));
                }
            }
        }
        if let Some(prism) = detect_prism_launch(exec) {
            match find_prism_instance(paths, &prism.instance_id) {
                Some(dir) => out.push(Diagnostic::info(format!(
                    "Prism-Instanz gefunden: {}",
                    prism_instance_label(&prism.instance_id, &dir)
                ))),
                None => out.push(Diagnostic::error(format!(
                    "Prism-Instanzordner '{}' wurde nicht gefunden. --launch erwartet die Instanz-ID (Ordnername), nicht den sichtbaren Namen. Geprüft: {}",
                    prism.instance_id,
                    prism_instance_dirs(paths)
                        .iter()
                        .map(|p| p.display().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))),
            }
            if prism.flatpak && !flatpak_app_installed(paths, "org.prismlauncher.PrismLauncher") {
                out.push(Diagnostic::warning(
                    "org.prismlauncher.PrismLauncher ist als Flatpak nicht installiert.",
                ));
            }
        }
        if exec.contains('%') {
            out.push(Diagnostic::info(
                "Exec enthält Desktop-Feldcodes (%f, %u …). GIO expandiert sie beim Start; CouchDeck übergibt die Datei unverändert an GIO.",
            ));
        }
    }
    if entry.dbus_activatable {
        out.push(Diagnostic::info(
            "DBusActivatable=true: Der Start erfolgt über die D-Bus-Aktivierung durch GIO.",
        ));
    }
    if entry.terminal {
        out.push(Diagnostic::warning(
            "Terminal=true: GIO öffnet den Eintrag in einem Terminal; im Fullscreen-Launcher ist das meist unerwünscht.",
        ));
    }
    if entry.hidden {
        out.push(Diagnostic::warning(
            "Hidden=true markiert den Eintrag laut Desktop-Spezifikation als gelöscht. Der Import ist möglich, der Start kann aber scheitern.",
        ));
    }
    if entry.no_display {
        out.push(Diagnostic::warning(
            "NoDisplay=true: Der Eintrag ist in Anwendungsmenüs normalerweise ausgeblendet.",
        ));
    }
    if !entry.warnings.is_empty() {
        for warning in &entry.warnings {
            out.push(Diagnostic::warning(warning.clone()));
        }
    }
    out
}

fn sanitize_file_component(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            out.push(ch);
        } else if ch.is_whitespace() || ch == '/' {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('.').trim_matches('-').to_owned();
    if trimmed.is_empty() {
        "entry".to_owned()
    } else {
        trimmed.chars().take(80).collect()
    }
}

/// Quote a single literal argument for the desktop-entry Exec field.
pub(crate) fn exec_quote(input: &str) -> String {
    let needs_quotes = input.is_empty()
        || input.chars().any(|c| {
            c.is_whitespace()
                || matches!(
                    c,
                    '"' | '\''
                        | '\\'
                        | '$'
                        | '`'
                        | ';'
                        | '&'
                        | '|'
                        | '<'
                        | '>'
                        | '('
                        | ')'
                        | '*'
                        | '?'
                        | '['
                        | ']'
                        | '#'
                        | '~'
                        | '%'
                )
        });
    let escaped = input
        .replace('%', "%%")
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('`', "\\`")
        .replace('$', "\\$");
    if needs_quotes {
        format!("\"{escaped}\"")
    } else {
        escaped
    }
}

fn managed_desktop_contents(name: &str, comment: &str, exec: &str, icon: &str) -> String {
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name={name}\n\
         Comment={comment}\n\
         Exec={exec}\n\
         Icon={icon}\n\
         Terminal=false\n\
         Categories=Game;\n\
         StartupNotify=true\n\
         X-CouchDeck-Managed=true\n"
    )
}

pub fn managed_steam_path(paths: &XdgPaths, app_id: &str) -> PathBuf {
    paths.managed_desktop_dir().join(format!(
        "couchdeck-steam-{}.desktop",
        sanitize_file_component(app_id)
    ))
}

// ---------------------------------------------------------------------------
// XDG autostart
// ---------------------------------------------------------------------------

/// Generate a **separate** managed desktop file for Flatpak Steam. The original
/// file is never modified. `app_id` must already have been validated as a
/// numeric Steam app id.
pub fn write_managed_steam(
    paths: &XdgPaths,
    app_id: &str,
    title: &str,
    icon: Option<&str>,
) -> Result<PathBuf, String> {
    let parsed: u32 = app_id
        .parse()
        .map_err(|_| format!("Ungültige Steam-App-ID: {app_id}"))?;
    if parsed == 0 {
        return Err("Steam-App-ID 0 ist ungültig.".into());
    }
    let path = managed_steam_path(paths, app_id);
    let contents = managed_desktop_contents(
        &escape_desktop_value(title),
        "Von CouchDeck verwalteter Steam-Flatpak-Start (Original unverändert)",
        &format!("flatpak run com.valvesoftware.Steam steam://rungameid/{app_id}"),
        icon.filter(|s| !s.trim().is_empty())
            .unwrap_or("com.valvesoftware.Steam"),
    );
    crate::storage::atomic_write(&path, contents.as_bytes(), false)
        .map_err(|err| format!("Verwaltete Steam-Datei konnte nicht geschrieben werden: {err}"))?;
    Ok(path)
}

pub fn managed_prism_path(paths: &XdgPaths, instance_id: &str) -> PathBuf {
    // The sanitized component alone can collide for different instance folder
    // names (spaces vs. hyphens). A short stable FNV-1a hash keeps managed
    // files distinct while remaining readable across Rust versions.
    let mut hash: u32 = 2_166_136_261;
    for byte in instance_id.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16_777_619);
    }
    paths.managed_desktop_dir().join(format!(
        "couchdeck-prism-{}-{hash:08x}.desktop",
        sanitize_file_component(instance_id)
    ))
}

/// Generate a managed desktop file that launches one Prism instance directly.
/// `instance_id` is the instance *folder* name, as required by `--launch`.
pub fn write_managed_prism(
    paths: &XdgPaths,
    instance_id: &str,
    title: &str,
    icon: Option<&str>,
) -> Result<PathBuf, String> {
    if instance_id.trim().is_empty()
        || instance_id.contains('/')
        || instance_id.contains('\\')
        || instance_id == "."
        || instance_id == ".."
        || instance_id.contains('\n')
    {
        return Err("Ungültige Prism-Instanz-ID (Ordnername erwartet).".into());
    }
    let path = managed_prism_path(paths, instance_id);
    let contents = managed_desktop_contents(
        &escape_desktop_value(title),
        "Von CouchDeck verwalteter Prism-Launcher-Instanzstart",
        &format!(
            "flatpak run org.prismlauncher.PrismLauncher --launch {}",
            exec_quote(instance_id)
        ),
        icon.filter(|s| !s.trim().is_empty())
            .unwrap_or("org.prismlauncher.PrismLauncher"),
    );
    crate::storage::atomic_write(&path, contents.as_bytes(), false)
        .map_err(|err| format!("Verwaltete Prism-Datei konnte nicht geschrieben werden: {err}"))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    #[test]
    fn parses_localized_name_and_escapes() {
        let entry = parse_desktop_text_with_locales(
            Path::new("/tmp/game.desktop"),
            "[Desktop Entry]\nType=Application\nName=Game\\sOne\nName[de]=Spiel Eins\nExec=foo --bar \"baz qux\"\nIcon=game\nCategories=Game;Action;\nNoDisplay=false\n",
            &["de".to_owned()],
        )
        .unwrap();
        assert_eq!(entry.name, "Spiel Eins");
        assert_eq!(entry.exec.as_deref(), Some("foo --bar \"baz qux\""));
        assert_eq!(entry.categories, vec!["Game", "Action"]);
        assert!(!entry.no_display);

        let base = parse_desktop_text_with_locales(
            Path::new("/tmp/game.desktop"),
            "[Desktop Entry]\nType=Application\nName=Game\\sOne\nName[de]=Spiel Eins\nExec=true\n",
            &[],
        )
        .unwrap();
        assert_eq!(base.name, "Game One");
    }

    #[test]
    fn localized_candidates_strip_encoding() {
        assert_eq!(
            locale_candidates_from("de_DE.UTF-8@euro"),
            vec!["de_DE".to_owned(), "de".to_owned()]
        );
        assert!(locale_candidates_from("C").is_empty());
    }

    #[test]
    fn rejects_non_application_types() {
        assert!(parse_desktop_text(
            Path::new("/tmp/link.desktop"),
            "[Desktop Entry]\nType=Link\nName=Link\n"
        )
        .is_err());
    }

    #[test]
    fn missing_group_is_error() {
        assert!(parse_desktop_text(Path::new("/tmp/x.desktop"), "Name=X\n").is_err());
    }

    #[test]
    fn discovery_skips_hidden_and_no_display() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = XdgPaths::from_env_isolated(tmp.path(), |key| match key {
            "XDG_DATA_HOME" => Some(OsString::from(tmp.path().join("data"))),
            "XDG_DATA_DIRS" => Some(OsString::from(tmp.path().join("sys"))),
            _ => None,
        })
        .unwrap();
        let apps = paths.data_home.join("applications");
        write(
            &apps.join("visible.desktop"),
            "[Desktop Entry]\nType=Application\nName=Visible\nExec=true\n",
        );
        write(
            &apps.join("hidden.desktop"),
            "[Desktop Entry]\nType=Application\nName=Hidden\nExec=true\nHidden=true\n",
        );
        write(
            &apps.join("nodisplay.desktop"),
            "[Desktop Entry]\nType=Application\nName=NoDisplay\nExec=true\nNoDisplay=true\n",
        );
        write(&apps.join("bad.desktop"), "not a desktop file\n");
        let found = discover_applications(&paths);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "Visible");
    }

    #[test]
    fn discovery_includes_nodisplay_but_marks_menu_visibility() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = XdgPaths::from_env_isolated(tmp.path(), |key| match key {
            "XDG_DATA_HOME" => Some(OsString::from(tmp.path().join("data"))),
            "XDG_DATA_DIRS" => Some(OsString::from(tmp.path().join("sys"))),
            _ => None,
        })
        .unwrap();
        let apps = paths.data_home.join("applications");
        write(
            &apps.join("visible.desktop"),
            "[Desktop Entry]\nType=Application\nName=Visible\nExec=true\n",
        );
        write(
            &apps.join("nodisplay.desktop"),
            "[Desktop Entry]\nType=Application\nName=NoDisplay\nExec=true\nNoDisplay=true\n",
        );
        write(
            &apps.join("hidden.desktop"),
            "[Desktop Entry]\nType=Application\nName=Hidden\nExec=true\nHidden=true\n",
        );
        write(
            &apps.join("missing-tryexec.desktop"),
            "[Desktop Entry]\nType=Application\nName=MissingTryExec\nExec=true\nTryExec=/definitely/not/here\n",
        );
        let entries = discover_entries(&paths, &[]);
        let names: Vec<_> = entries
            .entries
            .iter()
            .map(|entry| entry.file.name.as_str())
            .collect();
        assert!(names.contains(&"NoDisplay"));
        assert!(names.contains(&"Hidden"));
        assert!(names.contains(&"MissingTryExec"));
        let visible = entries
            .entries
            .iter()
            .find(|entry| entry.file.name == "Visible")
            .unwrap();
        assert!(visible.menu_visible);
        let nodisplay = entries
            .entries
            .iter()
            .find(|entry| entry.file.name == "NoDisplay")
            .unwrap();
        assert!(!nodisplay.menu_visible);
        let hidden = entries
            .entries
            .iter()
            .find(|entry| entry.file.name == "Hidden")
            .unwrap();
        assert!(!hidden.menu_visible);
        let missing = entries
            .entries
            .iter()
            .find(|entry| entry.file.name == "MissingTryExec")
            .unwrap();
        assert!(!missing.menu_visible);
        let menu = discover_applications(&paths);
        assert_eq!(menu.len(), 1);
        assert_eq!(menu[0].name, "Visible");
    }

    #[test]
    fn discovery_covers_flatpak_desktop_folder_managed_and_custom() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        let paths = XdgPaths::from_env_isolated(home, |key| match key {
            "XDG_CONFIG_HOME" => Some(OsString::from(home.join("config"))),
            "XDG_DATA_HOME" => Some(OsString::from(home.join("data"))),
            "XDG_DATA_DIRS" => Some(OsString::from(home.join("sys"))),
            _ => None,
        })
        .unwrap();
        std::fs::create_dir_all(home.join("config")).unwrap();
        std::fs::create_dir_all(home.join("Mein Desktop")).unwrap();
        std::fs::write(
            home.join("config/user-dirs.dirs"),
            "XDG_DESKTOP_DIR=\"$HOME/Mein Desktop\"\n",
        )
        .unwrap();
        let write_app = |path: &Path, name: &str| {
            write(
                path,
                &format!("[Desktop Entry]\nType=Application\nName={name}\nExec=true\n"),
            );
        };
        write_app(
            &paths
                .data_home
                .join("flatpak/exports/share/applications/flat.desktop"),
            "Flatpak User",
        );
        write_app(
            &home.join("Mein Desktop/desktop-folder.desktop"),
            "Desktop Folder",
        );
        write_app(
            &paths.managed_desktop_dir().join("managed.desktop"),
            "Managed",
        );
        let custom = home.join("custom apps");
        write_app(&custom.join("custom.desktop"), "Custom");
        let entries = discover_entries(&paths, std::slice::from_ref(&custom));
        let source = |name: &str| {
            entries
                .entries
                .iter()
                .find(|entry| entry.file.name == name)
                .map(|entry| entry.source)
                .unwrap()
        };
        assert_eq!(source("Flatpak User"), DiscoverySource::FlatpakUser);
        assert_eq!(source("Desktop Folder"), DiscoverySource::DesktopFolder);
        assert_eq!(source("Managed"), DiscoverySource::Managed);
        assert_eq!(source("Custom"), DiscoverySource::Custom);
        assert!(entries
            .roots
            .iter()
            .any(|root| root.path == custom && !root.automatic));
    }

    #[test]
    fn xdg_dir_order_and_subdirectory_desktop_ids_are_stable() {
        let tmp = tempfile::tempdir().unwrap();
        let one = tmp.path().join("one");
        let two = tmp.path().join("two");
        let paths = XdgPaths::from_env_isolated(tmp.path(), |key| match key {
            "XDG_DATA_HOME" => Some(OsString::from(tmp.path().join("data"))),
            "XDG_DATA_DIRS" => Some(OsString::from(format!(
                "{}:{}",
                one.display(),
                two.display()
            ))),
            _ => None,
        })
        .unwrap();
        write(
            &paths.data_home.join("applications/same.desktop"),
            "[Desktop Entry]\nType=Application\nName=User\nExec=true\n",
        );
        write(
            &one.join("applications/same.desktop"),
            "[Desktop Entry]\nType=Application\nName=FirstSystem\nExec=true\n",
        );
        write(
            &two.join("applications/same.desktop"),
            "[Desktop Entry]\nType=Application\nName=SecondSystem\nExec=true\n",
        );
        write(
            &one.join("applications/sub/Game With Space.desktop"),
            "[Desktop Entry]\nType=Application\nName=Sub Game\nExec=true\n",
        );
        let entries = discover_entries(&paths, &[]);
        assert_eq!(
            entries
                .entries
                .iter()
                .find(|entry| entry.desktop_id == "same.desktop")
                .unwrap()
                .file
                .name,
            "User"
        );
        assert!(entries
            .entries
            .iter()
            .any(|entry| entry.desktop_id == "sub-Game With Space.desktop"));
    }

    #[test]
    fn symlinked_desktop_files_work_and_symlink_loops_are_not_followed() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = XdgPaths::from_env_isolated(tmp.path(), |key| match key {
            "XDG_DATA_HOME" => Some(OsString::from(tmp.path().join("data"))),
            "XDG_DATA_DIRS" => Some(OsString::from(tmp.path().join("sys"))),
            _ => None,
        })
        .unwrap();
        let apps = paths.data_home.join("applications");
        write(
            &apps.join("real.desktop"),
            "[Desktop Entry]\nType=Application\nName=Real\nExec=true\n",
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink("real.desktop", apps.join("My Linked Game.desktop")).unwrap();
            symlink(&apps, apps.join("loop")).unwrap();
        }
        let entries = discover_entries(&paths, &[]);
        assert!(entries
            .entries
            .iter()
            .any(|entry| entry.file.name == "Real"));
        #[cfg(unix)]
        assert!(entries
            .entries
            .iter()
            .any(|entry| entry.desktop_id == "My Linked Game.desktop"));
    }

    #[test]
    fn onlyshow_notshow_and_tryexec_are_interpreted() {
        let file = parse_desktop_text(
            Path::new("/tmp/desktop.desktop"),
            "[Desktop Entry]\nType=Application\nName=Test\nExec=true\nOnlyShowIn=KDE;\n",
        )
        .unwrap();
        let mut desktops = BTreeSet::new();
        desktops.insert("KDE".to_owned());
        assert!(shown_in_desktops(&file, &desktops));
        desktops.clear();
        desktops.insert("GNOME".to_owned());
        assert!(!shown_in_desktops(&file, &desktops));
        let file = parse_desktop_text(
            Path::new("/tmp/desktop.desktop"),
            "[Desktop Entry]\nType=Application\nName=Test\nExec=true\nNotShowIn=GNOME;\n",
        )
        .unwrap();
        assert!(!shown_in_desktops(&file, &desktops));
    }

    #[test]
    fn user_entry_wins_over_system_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = XdgPaths::from_env_isolated(tmp.path(), |key| match key {
            "XDG_DATA_HOME" => Some(OsString::from(tmp.path().join("data"))),
            "XDG_DATA_DIRS" => Some(OsString::from(tmp.path().join("sys"))),
            _ => None,
        })
        .unwrap();
        write(
            &paths.data_home.join("applications/same.desktop"),
            "[Desktop Entry]\nType=Application\nName=User\nExec=true\n",
        );
        write(
            &paths.data_dirs[0].join("applications/same.desktop"),
            "[Desktop Entry]\nType=Application\nName=System\nExec=true\n",
        );
        let found = discover_applications(&paths);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "User");
    }

    #[test]
    fn steam_shortcut_detection() {
        let plain = detect_steam_shortcut("steam steam://rungameid/12345").unwrap();
        assert_eq!(plain.app_id, "12345");
        assert!(!plain.flatpak);
        let flatpak =
            detect_steam_shortcut("flatpak run com.valvesoftware.Steam steam://rungameid/777")
                .unwrap();
        assert!(flatpak.flatpak);
        assert_eq!(flatpak.app_id, "777");
        assert!(detect_steam_shortcut("steam steam://rungameid/0").is_none());
        assert!(detect_steam_shortcut("steam steam://rungameid/abc").is_none());
    }

    #[test]
    fn prism_launch_detection() {
        let prism = detect_prism_launch(
            "flatpak run org.prismlauncher.PrismLauncher --launch \"My Instance\"",
        )
        .unwrap();
        assert_eq!(prism.instance_id, "My Instance");
        assert!(prism.flatpak);
        assert!(detect_prism_launch("prism --launch ../../etc").is_none());
    }

    #[test]
    fn diagnostic_tokenizer_keeps_quoted_words_together() {
        assert_eq!(
            diagnostic_tokens("foo --bar \"baz qux\" 'literal'"),
            vec!["foo", "--bar", "baz qux", "'literal'"]
        );
    }

    #[test]
    fn managed_steam_file_is_separate_and_correct() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = XdgPaths::for_home(tmp.path());
        let original = tmp.path().join("original.desktop");
        write(
            &original,
            "[Desktop Entry]\nType=Application\nName=Steam Game\nExec=steam steam://rungameid/123\n",
        );
        let before = fs::read_to_string(&original).unwrap();
        let managed = write_managed_steam(&paths, "123", "Steam Game", None).unwrap();
        assert!(managed.starts_with(paths.managed_desktop_dir()));
        assert_eq!(fs::read_to_string(&original).unwrap(), before);
        let parsed = parse_desktop_file(&managed).unwrap();
        assert!(parsed
            .exec
            .as_deref()
            .unwrap()
            .contains("flatpak run com.valvesoftware.Steam steam://rungameid/123"));
        assert_eq!(
            fs::read_to_string(&managed)
                .unwrap()
                .matches("[Desktop Entry]")
                .count(),
            1
        );
    }

    #[test]
    fn managed_prism_quotes_spaces() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = XdgPaths::for_home(tmp.path());
        let managed = write_managed_prism(&paths, "My Instance", "Minecraft", None).unwrap();
        let parsed = parse_desktop_file(&managed).unwrap();
        assert_eq!(
            parsed.exec.as_deref(),
            Some("flatpak run org.prismlauncher.PrismLauncher --launch \"My Instance\"")
        );
        assert!(write_managed_prism(&paths, "../escape", "x", None).is_err());
    }

    #[test]
    fn icon_resolution_prefers_user_theme() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = XdgPaths::for_home(tmp.path());
        let icon = paths
            .data_home
            .join("icons/hicolor/128x128/apps/my-game.png");
        write(&icon, "not a real png");
        let mut resolver = IconResolver::new(&paths);
        assert_eq!(resolver.resolve("my-game"), Some(icon));
        assert_eq!(resolver.resolve("does-not-exist"), None);
    }

    #[test]
    fn prism_instance_lookup() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = XdgPaths::for_home(tmp.path());
        let dir = prism_instance_dirs(&paths)[0].join("MyInstance");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("instance.cfg"), "name=My World\n").unwrap();
        assert!(find_prism_instance(&paths, "MyInstance").is_some());
        let list = list_prism_instances(&paths, 10);
        assert_eq!(list.len(), 1);
        assert!(prism_instance_label("MyInstance", &dir).contains("My World"));
    }
}
