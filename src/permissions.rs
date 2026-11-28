//! Optional one-time setup for reading gamepad devices.
//!
//! The controller-home chord needs read access to `/dev/input/event*`. Many
//! systems grant this through an active-session `uaccess` ACL; otherwise the
//! user can be added to the `input` group. CapyDeck never elevates silently.

use std::io::IsTerminal;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputGroupStatus {
    Active,
    ConfiguredNeedsRelogin,
    Missing,
    GroupMissing,
    Unknown,
}

impl InputGroupStatus {
    pub const fn label_key(self) -> &'static str {
        match self {
            Self::Active => "permission-input-active",
            Self::ConfiguredNeedsRelogin => "permission-input-configured",
            Self::Missing => "permission-input-missing",
            Self::GroupMissing => "permission-input-group-missing",
            Self::Unknown => "permission-input-unknown",
        }
    }

    pub fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }
}

pub fn current_username() -> Result<String, String> {
    let output = Command::new("id")
        .arg("-un")
        .output()
        .map_err(|err| format!("Benutzername konnte nicht ermittelt werden: {err}"))?;
    if !output.status.success() {
        return Err("Benutzername konnte nicht ermittelt werden.".to_owned());
    }
    let name = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if name.is_empty() {
        return Err("Benutzername ist leer.".to_owned());
    }
    Ok(name)
}

fn groups_for(user: Option<&str>) -> Result<Vec<String>, String> {
    let mut command = Command::new("id");
    command.arg("-nG");
    if let Some(user) = user {
        command.arg(user);
    }
    let output = command
        .output()
        .map_err(|err| format!("Gruppen konnten nicht gelesen werden: {err}"))?;
    if !output.status.success() {
        return Err("Gruppen konnten nicht gelesen werden.".to_owned());
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .map(str::to_owned)
        .collect())
}

fn input_group_exists() -> bool {
    Command::new("getent")
        .args(["group", "input"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

pub fn input_group_status() -> InputGroupStatus {
    let Ok(current) = groups_for(None) else {
        return InputGroupStatus::Unknown;
    };
    if current.iter().any(|group| group == "input") {
        return InputGroupStatus::Active;
    }
    let configured = current_username()
        .ok()
        .and_then(|user| groups_for(Some(&user)).ok())
        .unwrap_or_default();
    if configured.iter().any(|group| group == "input") {
        return InputGroupStatus::ConfiguredNeedsRelogin;
    }
    if !input_group_exists() {
        return InputGroupStatus::GroupMissing;
    }
    InputGroupStatus::Missing
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupOutcome {
    AlreadyActive,
    AddedNeedsRelogin,
}

/// Add the current user to the `input` group after an explicit confirmation.
/// Uses `pkexec` when available (GUI Polkit prompt), otherwise `sudo` only when
/// the caller has a terminal. No shell is involved.
pub fn setup_input_group() -> Result<SetupOutcome, String> {
    match input_group_status() {
        InputGroupStatus::Active => return Ok(SetupOutcome::AlreadyActive),
        InputGroupStatus::ConfiguredNeedsRelogin => return Ok(SetupOutcome::AddedNeedsRelogin),
        InputGroupStatus::GroupMissing => {
            return Err("Die Gruppe 'input' existiert auf diesem System nicht.".to_owned())
        }
        InputGroupStatus::Unknown => {
            return Err("Die aktuellen Benutzergruppen konnten nicht gelesen werden.".to_owned())
        }
        InputGroupStatus::Missing => {}
    }

    let user = current_username()?;
    let mut command = if command_available("pkexec") {
        let mut command = Command::new("pkexec");
        command.args(["usermod", "-aG", "input", &user]);
        command
    } else if std::io::stdin().is_terminal() && command_available("sudo") {
        let mut command = Command::new("sudo");
        command.args(["usermod", "-aG", "input", &user]);
        command
    } else {
        return Err(format!(
            "Keine grafische oder terminalbasierte Rechteausweitung verfügbar. Bitte als Admin ausführen: sudo usermod -aG input {user}"
        ));
    };

    let output = command
        .output()
        .map_err(|err| format!("Rechteausweitung konnte nicht gestartet werden: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Benutzer konnte nicht zur Gruppe input hinzugefügt werden: {}",
            stderr.trim()
        ));
    }

    match input_group_status() {
        InputGroupStatus::Active => Ok(SetupOutcome::AlreadyActive),
        _ => Ok(SetupOutcome::AddedNeedsRelogin),
    }
}

fn command_available(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_labels_are_stable() {
        assert_eq!(
            InputGroupStatus::Active.label_key(),
            "permission-input-active"
        );
        assert!(InputGroupStatus::Active.is_active());
        assert!(!InputGroupStatus::Missing.is_active());
    }
}
