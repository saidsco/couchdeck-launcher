//! System-level actions and their asynchronous state.
//!
//! This module deliberately owns the `systemctl` integration and the power
//! state machine so the central application state machine does not grow with
//! every new system action.

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

    pub fn progress_key(self) -> &'static str {
        match self {
            Self::Reboot => "power-progress-reboot",
            Self::PowerOff => "power-progress-poweroff",
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
pub struct SystemOutcome {
    pub action: SystemAction,
    pub success: bool,
    pub message: String,
    pub duration: Duration,
}

pub struct SystemState {
    pub action: SystemAction,
    pub started: Instant,
    pub outcome: Option<SystemOutcome>,
    pub completed_at: Option<Instant>,
}

pub struct SystemController {
    rx: Option<Receiver<SystemOutcome>>,
    state: Option<SystemState>,
}

impl Default for SystemController {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemController {
    pub fn new() -> Self {
        Self {
            rx: None,
            state: None,
        }
    }

    pub fn start(&mut self, action: SystemAction) {
        crate::logging::log_info("power", action.label());
        self.state = Some(SystemState {
            action,
            started: Instant::now(),
            outcome: None,
            completed_at: None,
        });
        self.rx = Some(spawn(action));
    }

    pub fn state(&self) -> Option<&SystemState> {
        self.state.as_ref()
    }

    pub fn is_active(&self) -> bool {
        self.state.is_some()
    }

    /// Deliver at most one completed outcome. Failures clear the state; success
    /// keeps it visible until `tick` decides to close the launcher.
    pub fn poll(&mut self) -> Option<SystemOutcome> {
        let result = self.rx.as_ref().map(Receiver::try_recv);
        match result {
            Some(Ok(outcome)) => {
                self.rx = None;
                if outcome.success {
                    if let Some(state) = &mut self.state {
                        state.outcome = Some(outcome.clone());
                        state.completed_at = Some(Instant::now());
                    }
                } else {
                    self.state = None;
                }
                Some(outcome)
            }
            Some(Err(std::sync::mpsc::TryRecvError::Empty)) => None,
            Some(Err(std::sync::mpsc::TryRecvError::Disconnected)) => {
                self.rx = None;
                let action = self
                    .state
                    .as_ref()
                    .map(|state| state.action)
                    .unwrap_or(SystemAction::PowerOff);
                self.state = None;
                Some(SystemOutcome {
                    action,
                    success: false,
                    message: String::new(),
                    duration: Duration::ZERO,
                })
            }
            None => None,
        }
    }

    /// Returns true when the launcher should close after a successful system
    /// command. This is intentionally delayed briefly so the success message is
    /// visible before the process exits.
    pub fn tick(&mut self) -> bool {
        let Some(state) = &self.state else {
            return false;
        };
        let completed = state
            .completed_at
            .map(|at| at.elapsed() > Duration::from_millis(1400))
            .unwrap_or(false);
        if completed {
            self.state = None;
        }
        completed
    }
}

fn spawn(action: SystemAction) -> Receiver<SystemOutcome> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let started = Instant::now();
        let outcome = run(action, started);
        let _ = tx.send(outcome);
    });
    rx
}

fn run(action: SystemAction, started: Instant) -> SystemOutcome {
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
            return SystemOutcome {
                action,
                success: false,
                message: format!(
                    "`systemctl` konnte nicht gestartet werden: {err}. Auf Desktop-Distributionen mit systemd ist das normalerweise vorhanden."
                ),
                duration: started.elapsed(),
            }
        }
    };
    let output = child.wait_with_output();
    let duration = started.elapsed();
    match output {
        Ok(output) if output.status.success() => SystemOutcome {
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
            SystemOutcome {
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
        Err(err) => SystemOutcome {
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

    #[test]
    fn controller_starts_idle_and_tolerates_empty_poll() {
        let mut controller = SystemController::new();
        assert!(!controller.is_active());
        assert!(controller.state().is_none());
        assert!(controller.poll().is_none());
        assert!(!controller.tick());
    }
}
