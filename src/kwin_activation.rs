//! Optional KDE/KWin fallback for existing windows on Wayland.
//!
//! Winit cannot focus or unminimize an existing top-level on Wayland. The
//! standard path is xdg-activation-v1 with a token created by the caller. When
//! the caller has no token (for example a terminal command) or KWin rejects
//! the token, KWin scripting can activate our own window after an explicit user
//! request. This path is intentionally KDE-only and never affects other apps.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::paths::XdgPaths;

static KWIN_FALLBACK_RUNNING: AtomicBool = AtomicBool::new(false);

const SCRIPT: &str = r#"
var windows = (typeof workspace.windowList === "function") ? workspace.windowList() : workspace.clientList();
var target = null;
for (var i = 0; i < windows.length; ++i) {
    var w = windows[i];
    var app = w.desktopFileName || "";
    var cls = w.resourceClass || w.resourceName || "";
    if (app === "couchdeck" || app === "couchdeck.desktop" || cls === "couchdeck" || w.caption === "CouchDeck") {
        target = w;
        break;
    }
}
if (target !== null) {
    target.minimized = false;
    if (workspace.activeWindow !== undefined) {
        workspace.activeWindow = target;
    } else if (workspace.activeClient !== undefined) {
        workspace.activeClient = target;
    }
    // Some KWin/Wayland combinations acknowledge activation before the
    // minimized surface is mapped. Reassert once after the compositor has
    // processed the unminimize; this is a single timer, not a polling loop.
    var timer = new QTimer();
    timer.singleShot = true;
    timer.interval = 180;
    timer.timeout.connect(function() {
        target.minimized = false;
        if (workspace.activeWindow !== undefined) {
            workspace.activeWindow = target;
        } else if (workspace.activeClient !== undefined) {
            workspace.activeClient = target;
        }
    });
    timer.start();
}
"#;

struct RunningGuard;

impl Drop for RunningGuard {
    fn drop(&mut self) {
        KWIN_FALLBACK_RUNNING.store(false, Ordering::Release);
    }
}

/// Schedule one best-effort KWin activation attempt in the background.
///
/// The delayed attempt is skipped when the shared focus state reports a real
/// focus event in the meantime. Returns `false` if another fallback is already
/// scheduled/running. Starting a script is not proof that KWin activated the
/// window; only an observed focus event is.
pub fn activate_after(paths: XdgPaths, delay: Duration, focus_state: Arc<AtomicBool>) -> bool {
    if KWIN_FALLBACK_RUNNING
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return false;
    }
    thread::spawn(move || {
        let _guard = RunningGuard;
        if !delay.is_zero() {
            thread::sleep(delay);
        }
        if focus_state.load(Ordering::Acquire) {
            crate::logging::log_info(
                "kwin",
                "KWin-Fallback nicht nötig; Fensterfokus wurde bereits beobachtet.",
            );
            return;
        }
        if let Err(err) = activate(paths) {
            crate::logging::log_warn("kwin", format!("KWin-Aktivierung fehlgeschlagen: {err}"));
        } else {
            crate::logging::log_info(
                "kwin",
                "KWin-Aktivierungsskript wurde geladen und gestartet; Fokus wird beobachtet.",
            );
            thread::sleep(Duration::from_millis(1200));
            if !focus_state.load(Ordering::Acquire) {
                crate::logging::log_warn(
                    "kwin",
                    "KWin-Skript gestartet, aber innerhalb von 1,2 s kein Fensterfokus beobachtet.",
                );
            }
        }
    });
    true
}

fn activate(paths: XdgPaths) -> Result<(), String> {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| paths.state_dir());
    fs::create_dir_all(&dir).map_err(|err| format!("Verzeichnis {}: {err}", dir.display()))?;

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let plugin = format!("couchdeck-activate-{}-{nonce}", std::process::id());
    let script_path = dir.join(format!("{plugin}.js"));
    fs::write(&script_path, SCRIPT)
        .map_err(|err| format!("Skript {}: {err}", script_path.display()))?;
    let _ = fs::set_permissions(&script_path, fs::Permissions::from_mode(0o600));

    let result = (|| {
        let qdbus = find_qdbus()?;
        let script = script_path
            .to_str()
            .ok_or_else(|| "Skriptpfad ist nicht UTF-8".to_owned())?;
        run_qdbus(
            qdbus,
            &[
                "org.kde.KWin",
                "/Scripting",
                "org.kde.kwin.Scripting.loadScript",
                script,
                &plugin,
            ],
        )?;
        run_qdbus(
            qdbus,
            &["org.kde.KWin", "/Scripting", "org.kde.kwin.Scripting.start"],
        )?;
        // Give KWin time to execute the one-shot script before unloading it.
        thread::sleep(Duration::from_millis(300));
        let _ = run_qdbus(
            qdbus,
            &[
                "org.kde.KWin",
                "/Scripting",
                "org.kde.kwin.Scripting.unloadScript",
                &plugin,
            ],
        );
        Ok(())
    })();

    let _ = fs::remove_file(&script_path);
    result
}

fn find_qdbus() -> Result<&'static str, String> {
    for candidate in ["qdbus6", "qdbus-qt6", "qdbus"] {
        if command_available(candidate) {
            return Ok(candidate);
        }
    }
    Err("qdbus6/qdbus wurde nicht gefunden; KWin-Fallback nicht verfügbar".to_owned())
}

fn command_available(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn run_qdbus(name: &str, args: &[&str]) -> Result<(), String> {
    let output = Command::new(name)
        .args(args)
        .output()
        .map_err(|err| format!("{name} konnte nicht gestartet werden: {err}"))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(format!(
        "{name} {}: {}",
        args.get(2).copied().unwrap_or("Aufruf"),
        stderr.trim()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_script_matches_only_couchdeck() {
        assert!(SCRIPT.contains("app === \"couchdeck\""));
        assert!(SCRIPT.contains("w.caption === \"CouchDeck\""));
        assert!(SCRIPT.contains("target.minimized = false"));
        assert!(SCRIPT.contains("workspace.activeWindow = target"));
    }
}
