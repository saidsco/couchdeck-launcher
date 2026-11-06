//! System power actions.
//!
//! CouchDeck deliberately uses fixed, argument-separated `systemctl` calls and
//! never a shell. Polkit/logind decides whether the active session is allowed
//! to reboot or power off; failures are reported in the UI.

use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemAction {
    Reboot,
    PowerOff,
}

impl SystemAction {
    pub fn label(self) -> &'static str {
        match self {
            Self::Reboot => "System neu starten",
            Self::PowerOff => "System herunterfahren",
        }
    }

    pub fn confirm_title(self) -> &'static str {
        match self {
            Self::Reboot => "System neu starten?",
            Self::PowerOff => "System herunterfahren?",
        }
    }

    pub fn confirm_label(self) -> &'static str {
        match self {
            Self::Reboot => "Neu starten",
            Self::PowerOff => "Herunterfahren",
        }
    }

    pub fn progress(self) -> &'static str {
        match self {
            Self::Reboot => "System wird neu gestartet …",
            Self::PowerOff => "System wird heruntergefahren …",
        }
    }

    pub fn argument(self) -> &'static str {
        match self {
            Self::Reboot => "reboot",
            Self::PowerOff => "poweroff",
        }
    }

    pub fn command_args(self) -> [&'static str; 3] {
        ["--no-block", "--no-wall", self.argument()]
    }
}

#[derive(Debug, Clone)]
pub struct PowerOutcome {
    pub action: SystemAction,
    pub success: bool,
    pub message: String,
    pub duration: Duration,
}

pub fn spawn(action: SystemAction) -> Receiver<PowerOutcome> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let started = Instant::now();
        let outcome = run(action, started);
        let _ = tx.send(outcome);
    });
    rx
}

fn run(action: SystemAction, started: Instant) -> PowerOutcome {
    let args = action.command_args();
    let result = Command::new("systemctl")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let child = match result {
        Ok(child) => child,
        Err(err) => {
            return PowerOutcome {
                action,
                success: false,
                message: format!(
                    "`systemctl` konnte nicht gestartet werden: {err}. Auf Bazzite ist systemd normalerweise vorhanden."
                ),
                duration: started.elapsed(),
            }
        }
    };
    let output = child.wait_with_output();
    let duration = started.elapsed();
    match output {
        Ok(output) if output.status.success() => PowerOutcome {
            action,
            success: true,
            message: format!(
                "Befehl '{}' wurde an systemd übergeben. Die Sitzung wird beendet.",
                action.label()
            ),
            duration,
        },
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let detail = stderr.trim();
            PowerOutcome {
                action,
                success: false,
                message: if detail.is_empty() {
                    format!(
                        "systemctl ist mit Status {} fehlgeschlagen. Fehlt die Polkit-Berechtigung für diese Sitzung?",
                        output.status
                    )
                } else {
                    format!(
                        "systemctl ist mit Status {} fehlgeschlagen:\n{detail}",
                        output.status
                    )
                },
                duration,
            }
        }
        Err(err) => PowerOutcome {
            action,
            success: false,
            message: format!("systemctl konnte nicht abgeschlossen werden: {err}"),
            duration,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_commands_are_fixed_and_shell_free() {
        assert_eq!(SystemAction::PowerOff.argument(), "poweroff");
        assert_eq!(SystemAction::Reboot.argument(), "reboot");
        assert_eq!(
            SystemAction::PowerOff.command_args(),
            ["--no-block", "--no-wall", "poweroff"]
        );
        assert!(!SystemAction::Reboot.label().is_empty());
        assert!(!SystemAction::PowerOff.progress().is_empty());
    }
}
