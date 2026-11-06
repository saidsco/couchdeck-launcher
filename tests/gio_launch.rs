//! Integration test for the only launch path: `gio launch`.
//!
//! This test launches `/bin/true` through GIO. It is skipped automatically when
//! `gio` is not installed.

use std::fs;
use std::time::Duration;

use couchdeck::desktop::gio_available;
use couchdeck::launch::spawn_gio_launch;
use couchdeck::paths::XdgPaths;

#[test]
fn gio_launches_desktop_file_with_spaces_and_quoted_exec() {
    if !gio_available() {
        eprintln!("gio fehlt – Integrationstest übersprungen");
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let paths = XdgPaths::for_home(tmp.path());
    let dir = tmp.path().join("Ordner mit Leerzeichen");
    fs::create_dir_all(&dir).unwrap();
    let desktop = dir.join("Spiel mit Leerzeichen.desktop");
    fs::write(
        &desktop,
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=GIO Integrationstest\n\
         Exec=/bin/true \"argument with spaces\" --level=2\n\
         Terminal=false\n",
    )
    .unwrap();

    let rx = spawn_gio_launch(desktop, "GIO Integrationstest".to_owned(), paths);
    let outcome = rx
        .recv_timeout(Duration::from_secs(15))
        .expect("gio launch should answer");
    assert!(outcome.success, "gio launch failed: {}", outcome.message);
}

#[test]
fn gio_reports_missing_desktop_file() {
    if !gio_available() {
        eprintln!("gio fehlt – Integrationstest übersprungen");
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let paths = XdgPaths::for_home(tmp.path());
    let rx = spawn_gio_launch(
        tmp.path().join("does-not-exist.desktop"),
        "Fehlt".to_owned(),
        paths,
    );
    let outcome = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("validation should answer");
    assert!(!outcome.success);
    assert!(outcome.message.contains("nicht gefunden") || outcome.message.contains("fehlt"));
}
