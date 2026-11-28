use std::path::PathBuf;
use std::process::ExitCode;

use capydeck::app::{AppOptions, CapyDeckApp};
use capydeck::paths::XdgPaths;
use capydeck::{activation, home, instance, logging, permissions, session, storage};

const USAGE: &str = "\
CapyDeck – familienfreundlicher, controller-first Spiele-Launcher

VERWENDUNG:
    capydeck [OPTIONEN]
    capydeck --activate
    capydeck --home

OPTIONEN:
    --windowed              Fenster statt Vollbild (Entwicklung)
    --no-controller         Gamepad-Thread deaktivieren (Diagnose)
    --autostart             Wird vom XDG-Autostart-Eintrag verwendet
    --activate              Laufende CapyDeck-Instanz um Fensteraktivierung bitten
    --home                  CapyDeck aktivieren oder starten, falls nicht laufend
    --install-home-launcher Optionalen „CapyDeck Home“-Menüeintrag anlegen
    --remove-home-launcher  Optionalen Home-Menüeintrag entfernen
    --setup-input-permissions  Benutzer zur Gruppe input hinzufügen (Controller-Home)
    --yes                   Bestätigung für --setup-input-permissions überspringen
    --diagnose-window       Fokus-/Minimierungswechsel ausführlicher loggen
    --version               Version anzeigen
    -h, --help              Diese Hilfe anzeigen

Beim Start wird die Bibliothek des zuletzt aktiven Profils geöffnet.
Spiele werden ausschließlich über `gio launch <absolute-datei.desktop>` gestartet.
";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("CapyDeck: {err}");
            logging::log_error("main", &err);
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut windowed = false;
    let mut no_controller = false;
    let mut autostart_launch = false;
    let mut activate = false;
    let mut home_mode = false;
    let mut install_home_launcher = false;
    let mut remove_home_launcher = false;
    let mut setup_input_permissions = false;
    let mut assume_yes = false;
    let mut diagnose_window = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--windowed" => windowed = true,
            "--no-controller" => no_controller = true,
            "--autostart" => autostart_launch = true,
            "--activate" => activate = true,
            "--home" => home_mode = true,
            "--install-home-launcher" => install_home_launcher = true,
            "--remove-home-launcher" => remove_home_launcher = true,
            "--setup-input-permissions" => setup_input_permissions = true,
            "--yes" => assume_yes = true,
            "--diagnose-window" => diagnose_window = true,
            "--version" | "-V" => {
                println!("capydeck {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--help" | "-h" => {
                println!("{USAGE}");
                return Ok(());
            }
            other => return Err(format!("Unbekannte Option '{other}'.\n\n{USAGE}")),
        }
    }

    let launcher_actions = install_home_launcher as u8 + remove_home_launcher as u8;
    if (activate && home_mode)
        || (launcher_actions > 0 && (activate || home_mode || setup_input_permissions))
        || launcher_actions > 1
        || (setup_input_permissions && (activate || home_mode || launcher_actions > 0))
        || (assume_yes && !setup_input_permissions)
    {
        return Err(format!(
            "--activate, --home, --install-home-launcher und --remove-home-launcher können nicht kombiniert werden.\n\n{USAGE}"
        ));
    }

    let paths = XdgPaths::discover().map_err(|err| err.to_string())?;
    logging::init(&paths.log_path());
    logging::install_panic_hook();
    let session_info = session::detect_current();
    logging::log_info(
        "main",
        format!(
            "capydeck {} | Session={} | Wayland={} | Desktop={:?} | DesktopTokens={:?} | XDG_DATA_HOME={} | Autostart={} | Diagnose={}",
            env!("CARGO_PKG_VERSION"),
            std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unbekannt".into()),
            std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "nicht gesetzt".into()),
            session_info.environment,
            session_info.tokens,
            paths.data_home.display(),
            autostart_launch,
            diagnose_window
        ),
    );
    if setup_input_permissions {
        return setup_input_permissions_cli(assume_yes);
    }
    if install_home_launcher {
        let executable = home_executable()?;
        let launcher = home::install_launcher(&paths, &executable)?;
        logging::log_info(
            "home",
            format!(
                "Home-Launcher angelegt: {} (Exec={})",
                launcher.display(),
                executable.display()
            ),
        );
        println!("Home-Launcher angelegt: {}", launcher.display());
        return Ok(());
    }
    if remove_home_launcher {
        let removed = home::remove_launcher(&paths)?;
        if removed {
            logging::log_info("home", "Home-Launcher entfernt.");
            println!("Home-Launcher entfernt.");
        } else {
            logging::log_info("home", "Kein Home-Launcher vorhanden.");
            println!("Kein Home-Launcher vorhanden.");
        }
        return Ok(());
    }
    if home_mode {
        let result = activation::send_activation(&paths);
        logging::log_info(
            "activation",
            format!("CLI --home: {:?} - {}", result, result.user_message()),
        );
        if result.is_success() {
            println!("{}", result.user_message());
            return Ok(());
        }
        if !result.is_not_running() {
            return Err(result.user_message());
        }
        logging::log_info(
            "activation",
            "CLI --home: keine laufende Instanz; CapyDeck wird normal gestartet.",
        );
    }
    if activate {
        let result = activation::send_activation(&paths);
        logging::log_info(
            "activation",
            format!("CLI --activate: {:?} - {}", result, result.user_message()),
        );
        if result.is_success() {
            println!("{}", result.user_message());
            return Ok(());
        }
        return Err(result.user_message());
    }
    storage::ensure_directories(&paths)?;
    // A token from the environment is only meaningful for a window that is
    // being created. CapyDeck's window already exists here; do not leak the
    // one-shot token into games/launchers started later.
    if std::env::var_os("XDG_ACTIVATION_TOKEN").is_some()
        || std::env::var_os("DESKTOP_STARTUP_ID").is_some()
    {
        logging::log_info(
            "activation",
            "Start-Activation-Token wird nach dem Fensterstart nicht an Kindprozesse weitergegeben.",
        );
        std::env::remove_var("XDG_ACTIVATION_TOKEN");
        std::env::remove_var("DESKTOP_STARTUP_ID");
    }

    let _instance_guard = match instance::InstanceGuard::acquire(&paths.instance_lock_path()) {
        Ok(guard) => guard,
        Err(instance::InstanceError::AlreadyRunning) => {
            logging::log_info(
                "main",
                "CapyDeck läuft bereits; zweiter Start wird beendet.",
            );
            println!("CapyDeck läuft bereits.");
            return Ok(());
        }
        Err(err) => return Err(err.to_string()),
    };

    let options = AppOptions {
        windowed,
        no_controller,
        diagnose_window,
    };
    let paths_for_app = paths.clone();
    let viewport = eframe::egui::ViewportBuilder::default()
        .with_title("CapyDeck")
        .with_app_id("capydeck")
        .with_inner_size([1600.0, 900.0])
        .with_min_inner_size([960.0, 540.0])
        .with_decorations(windowed)
        .with_fullscreen(!windowed);

    let native_options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        vsync: true,
        ..Default::default()
    };

    let result = eframe::run_native(
        "CapyDeck",
        native_options,
        Box::new(move |cc| {
            Ok(
                Box::new(CapyDeckApp::new(cc, paths_for_app.clone(), options))
                    as Box<dyn eframe::App>,
            )
        }),
    );
    result.map_err(|err| format!("Fenster/Start fehlgeschlagen: {err}"))
}

fn setup_input_permissions_cli(assume_yes: bool) -> Result<(), String> {
    use std::io::IsTerminal;

    let status = permissions::input_group_status();
    println!(
        "Controller-Home / /dev/input: {}",
        match status {
            permissions::InputGroupStatus::Active => "Gruppe input ist aktiv.",
            permissions::InputGroupStatus::ConfiguredNeedsRelogin =>
                "Gruppe input ist zugewiesen; bitte ab- und wieder anmelden.",
            permissions::InputGroupStatus::Missing => "Gruppe input fehlt für diesen Benutzer.",
            permissions::InputGroupStatus::GroupMissing => "Die Gruppe input existiert nicht.",
            permissions::InputGroupStatus::Unknown => "Status unbekannt.",
        }
    );

    match status {
        permissions::InputGroupStatus::Active => return Ok(()),
        permissions::InputGroupStatus::ConfiguredNeedsRelogin => {
            println!(
                "Bitte einmal abmelden und neu anmelden; danach ist Controller-Home verfügbar."
            );
            return Ok(());
        }
        permissions::InputGroupStatus::GroupMissing | permissions::InputGroupStatus::Unknown => {
            return Err(
                "Die input-Gruppe kann auf diesem System nicht eingerichtet werden.".to_owned(),
            )
        }
        permissions::InputGroupStatus::Missing => {}
    }

    if !assume_yes {
        if !std::io::stdin().is_terminal() {
            return Err(
                "Kein Terminal für die Bestätigung. Aufruf mit --setup-input-permissions --yes oder manuell als Admin: sudo usermod -aG input <benutzer>".to_owned(),
            );
        }
        println!(
            "CapyDeck benötigt Lesezugriff auf Gamepad-Geräte. Dafür wird der Benutzer zur Gruppe 'input' hinzugefügt.\nDies ist eine einmalige Systemänderung und erfordert Admin-Rechte. Danach ist eine Neuanmeldung nötig."
        );
        println!("Jetzt hinzufügen? [y/N] ");
        let mut answer = String::new();
        std::io::stdin()
            .read_line(&mut answer)
            .map_err(|err| format!("Bestätigung konnte nicht gelesen werden: {err}"))?;
        if !matches!(
            answer.trim().to_ascii_lowercase().as_str(),
            "y" | "yes" | "j" | "ja"
        ) {
            println!("Abgebrochen; keine Systemänderung vorgenommen.");
            return Ok(());
        }
    }

    match permissions::setup_input_group()? {
        permissions::SetupOutcome::AlreadyActive => {
            println!("Gruppe input ist bereits aktiv.");
        }
        permissions::SetupOutcome::AddedNeedsRelogin => {
            println!("Benutzer wurde zur Gruppe input hinzugefügt. Bitte ab- und wieder anmelden.");
        }
    }
    Ok(())
}

fn home_executable() -> Result<PathBuf, String> {
    // AppImage exposes its persistent outer path; current_exe() would point to
    // the temporary extraction directory. Prefer APPIMAGE for desktop entries.
    if let Some(appimage) = std::env::var_os("APPIMAGE").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(appimage));
    }
    let executable = std::env::current_exe().map_err(|err| format!("Eigener Binarypfad: {err}"))?;
    Ok(executable.canonicalize().unwrap_or(executable))
}
