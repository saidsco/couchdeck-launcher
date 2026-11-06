//! Launching through GIO.
//!
//! `gio launch <absolute path>` is the only launch mechanism. CouchDeck never
//! splits, evaluates or shell-interprets the `Exec=` value. A successful
//! `gio launch` means "start request accepted", never "game is running" or
//! "game has exited".

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use crate::desktop::{analyze_entry, parse_desktop_file, Severity};

#[derive(Debug, Clone)]
pub struct LaunchOutcome {
    pub title: String,
    pub success: bool,
    pub message: String,
    pub duration: Duration,
}

#[derive(Debug, Clone)]
pub struct LaunchError(pub String);

impl std::fmt::Display for LaunchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Validate enough to give a useful error before handing the file to GIO.
/// `Exec` is deliberately not parsed for execution.
pub fn validate_for_launch(path: &Path, paths: &crate::paths::XdgPaths) -> Result<(), LaunchError> {
    if !path.is_absolute() {
        return Err(LaunchError(format!(
            "Start nur mit absolutem Desktop-Dateipfad möglich: {}",
            path.display()
        )));
    }
    if !path.is_file() {
        return Err(LaunchError(format!(
            "Desktop-Datei wurde nicht gefunden: {}",
            path.display()
        )));
    }
    let entry = parse_desktop_file(path)
        .map_err(|err| LaunchError(format!("Desktop-Datei ist ungültig: {err}")))?;
    if !entry.executable_for_gio() {
        return Err(LaunchError(
            "Desktop-Datei enthält weder Exec= noch DBusActivatable=true.".into(),
        ));
    }
    let errors: Vec<String> = analyze_entry(&entry, paths)
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.message)
        .collect();
    if !errors.is_empty() {
        return Err(LaunchError(format!(
            "Der Eintrag kann wahrscheinlich nicht starten:\n{}",
            errors.join("\n")
        )));
    }
    Ok(())
}

/// Start `gio launch` in a background thread so a slow GIO call never freezes
/// the launcher. The result is delivered asynchronously.
pub fn spawn_gio_launch(
    path: PathBuf,
    title: String,
    paths: crate::paths::XdgPaths,
) -> Receiver<LaunchOutcome> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let started = Instant::now();
        let outcome = match validate_for_launch(&path, &paths) {
            Err(err) => LaunchOutcome {
                title,
                success: false,
                message: err.0,
                duration: started.elapsed(),
            },
            Ok(()) => run_gio(&path, &title, started),
        };
        let _ = tx.send(outcome);
    });
    rx
}

fn run_gio(path: &Path, title: &str, started: Instant) -> LaunchOutcome {
    let child = match Command::new("gio")
        .arg("launch")
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(err) => {
            return LaunchOutcome {
                title: title.to_owned(),
                success: false,
                message: format!(
                    "gio konnte nicht gestartet werden: {err}. Ist GLib/GIO installiert?"
                ),
                duration: started.elapsed(),
            }
        }
    };

    // `wait_with_output` drains stdout/stderr concurrently on Unix, avoiding
    // pipe-buffer deadlocks on chatty launchers.
    let output = child.wait_with_output();
    let duration = started.elapsed();
    match output {
        Ok(output) if output.status.success() => LaunchOutcome {
            title: title.to_owned(),
            success: true,
            message: format!(
                "Startbefehl an GIO übergeben ({}). CouchDeck erkennt nicht automatisch, wann das Spiel endet.",
                output.status
            ),
            duration,
        },
        Ok(output) => {
            let detail = String::from_utf8_lossy(&output.stderr);
            let detail = detail.trim();
            LaunchOutcome {
                title: title.to_owned(),
                success: false,
                message: if detail.is_empty() {
                    format!(
                        "gio launch ist mit Status {} fehlgeschlagen.",
                        output.status
                    )
                } else {
                    format!(
                        "gio launch ist mit Status {} fehlgeschlagen:\n{detail}",
                        output.status
                    )
                },
                duration,
            }
        }
        Err(err) => LaunchOutcome {
            title: title.to_owned(),
            success: false,
            message: format!("gio launch konnte nicht abgeschlossen werden: {err}"),
            duration,
        },
    }
}

/// Convenience used by the dependency check screen / logs.
pub fn gio_version() -> Result<String, String> {
    let output = Command::new("gio")
        .arg("--version")
        .output()
        .map_err(|err| format!("gio nicht gefunden: {err}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    } else {
        Err(format!(
            "gio --version fehlgeschlagen: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::XdgPaths;
    use std::fs;

    #[test]
    fn rejects_missing_and_relative_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = XdgPaths::for_home(tmp.path());
        assert!(validate_for_launch(Path::new("relative.desktop"), &paths).is_err());
        assert!(validate_for_launch(&tmp.path().join("missing.desktop"), &paths).is_err());
    }

    #[test]
    fn accepts_exec_with_spaces_without_interpreting_it() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = XdgPaths::for_home(tmp.path());
        let path = tmp.path().join("quoted.desktop");
        fs::write(
            &path,
            "[Desktop Entry]\nType=Application\nName=Quoted\nExec=/bin/echo \"hello world\" --arg\n",
        )
        .unwrap();
        validate_for_launch(&path, &paths).unwrap();
    }

    #[test]
    fn rejects_missing_exec() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = XdgPaths::for_home(tmp.path());
        let path = tmp.path().join("noexec.desktop");
        fs::write(&path, "[Desktop Entry]\nType=Application\nName=No Exec\n").unwrap();
        assert!(validate_for_launch(&path, &paths).is_err());
    }
}
