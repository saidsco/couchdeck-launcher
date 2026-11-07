use std::process::ExitCode;

use couchdeck::app::{AppOptions, CouchDeckApp};
use couchdeck::paths::XdgPaths;
use couchdeck::{instance, logging, storage};

const USAGE: &str = "\
CouchDeck – controller-first Fullscreen-Spielebibliothek

VERWENDUNG:
    couchdeck [OPTIONEN]

OPTIONEN:
    --windowed        Fenster statt Vollbild (Entwicklung)
    --no-controller   Gamepad-Thread deaktivieren (Diagnose)
    --autostart       Wird vom XDG-Autostart-Eintrag verwendet
    --version         Version anzeigen
    -h, --help        Diese Hilfe anzeigen

Beim Start wird die Bibliothek des zuletzt aktiven Profils geöffnet.
Spiele werden ausschließlich über `gio launch <absolute-datei.desktop>` gestartet.
";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("CouchDeck: {err}");
            logging::log_error("main", &err);
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut windowed = false;
    let mut no_controller = false;
    let mut autostart_launch = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--windowed" => windowed = true,
            "--no-controller" => no_controller = true,
            "--autostart" => autostart_launch = true,
            "--version" | "-V" => {
                println!("couchdeck {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--help" | "-h" => {
                println!("{USAGE}");
                return Ok(());
            }
            other => return Err(format!("Unbekannte Option '{other}'.\n\n{USAGE}")),
        }
    }

    let paths = XdgPaths::discover().map_err(|err| err.to_string())?;
    logging::init(&paths.log_path());
    logging::install_panic_hook();
    logging::log_info(
        "main",
        format!(
            "couchdeck {} | Session={} | Wayland={} | XDG_DATA_HOME={} | Autostart={}",
            env!("CARGO_PKG_VERSION"),
            std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unbekannt".into()),
            std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "nicht gesetzt".into()),
            paths.data_home.display(),
            autostart_launch
        ),
    );
    storage::ensure_directories(&paths)?;

    let _instance_guard = match instance::InstanceGuard::acquire(&paths.instance_lock_path()) {
        Ok(guard) => guard,
        Err(instance::InstanceError::AlreadyRunning) => {
            logging::log_info(
                "main",
                "CouchDeck läuft bereits; zweiter Start wird beendet.",
            );
            println!("CouchDeck läuft bereits.");
            return Ok(());
        }
        Err(err) => return Err(err.to_string()),
    };

    let options = AppOptions {
        windowed,
        no_controller,
    };
    let paths_for_app = paths.clone();
    let viewport = eframe::egui::ViewportBuilder::default()
        .with_title("CouchDeck")
        .with_app_id("couchdeck")
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
        "CouchDeck",
        native_options,
        Box::new(move |cc| {
            Ok(
                Box::new(CouchDeckApp::new(cc, paths_for_app.clone(), options))
                    as Box<dyn eframe::App>,
            )
        }),
    );
    result.map_err(|err| format!("Fenster/Start fehlgeschlagen: {err}"))
}
