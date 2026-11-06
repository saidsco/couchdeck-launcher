//! Daily play-time accounting for child profiles.
//!
//! First stage: active profile time is measured while the CouchDeck process is
//! running. Running games are not force-closed; only new launches are blocked.
//! The state is intentionally separate from `library.json` because it is
//! updated frequently.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::paths::XdgPaths;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileTime {
    /// Local calendar day (`YYYY-MM-DD`).
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub used_seconds: u64,
    #[serde(default)]
    pub bonus_minutes: u32,
    #[serde(default)]
    pub warned_5: bool,
    #[serde(default)]
    pub warned_1: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timekeeping {
    #[serde(default = "schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub profiles: BTreeMap<String, ProfileTime>,
}

fn schema_version() -> u32 {
    SCHEMA_VERSION
}

impl Default for Timekeeping {
    fn default() -> Self {
        Self::new()
    }
}

impl Timekeeping {
    pub fn new() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            profiles: BTreeMap::new(),
        }
    }

    pub fn normalize(&mut self) {
        if self.schema_version > SCHEMA_VERSION {
            self.schema_version = SCHEMA_VERSION;
        }
    }

    fn ensure_date(&mut self, profile_id: &str, today: &str) -> &mut ProfileTime {
        let entry = self.profiles.entry(profile_id.to_owned()).or_default();
        if entry.date != today {
            *entry = ProfileTime {
                date: today.to_owned(),
                ..Default::default()
            };
        }
        entry
    }

    pub fn status(
        &mut self,
        profile_id: &str,
        daily_limit_minutes: Option<u32>,
        today: &str,
    ) -> TimeStatus {
        let state = self.ensure_date(profile_id, today);
        let used_seconds = state.used_seconds;
        let bonus_minutes = state.bonus_minutes;
        match daily_limit_minutes {
            None => TimeStatus {
                limited: false,
                used_seconds,
                bonus_minutes,
                remaining_seconds: 0,
                remaining_minutes: 0,
                exhausted: false,
            },
            Some(limit_minutes) => {
                let total = u64::from(limit_minutes).saturating_add(u64::from(bonus_minutes)) * 60;
                let remaining_seconds = total.saturating_sub(used_seconds);
                TimeStatus {
                    limited: true,
                    used_seconds: used_seconds.min(total),
                    bonus_minutes,
                    remaining_seconds,
                    remaining_minutes: remaining_seconds.div_ceil(60),
                    exhausted: remaining_seconds == 0,
                }
            }
        }
    }

    /// Accumulate at most the remaining budget. Once exhausted, the clock stops
    /// instead of silently consuming future extra-time grants.
    pub fn add_usage(
        &mut self,
        profile_id: &str,
        seconds: u64,
        daily_limit_minutes: Option<u32>,
        today: &str,
    ) -> TimeStatus {
        let Some(limit_minutes) = daily_limit_minutes else {
            return self.status(profile_id, None, today);
        };
        let state = self.ensure_date(profile_id, today);
        let total = u64::from(limit_minutes).saturating_add(u64::from(state.bonus_minutes)) * 60;
        let remaining = total.saturating_sub(state.used_seconds);
        if remaining > 0 {
            state.used_seconds = state
                .used_seconds
                .saturating_add(seconds.min(remaining))
                .min(total);
        }
        self.status(profile_id, Some(limit_minutes), today)
    }

    pub fn add_extra_minutes(&mut self, profile_id: &str, minutes: u32, today: &str) {
        let state = self.ensure_date(profile_id, today);
        state.bonus_minutes = state.bonus_minutes.saturating_add(minutes);
        state.warned_5 = false;
        state.warned_1 = false;
    }

    pub fn reset_extra_minutes(&mut self, profile_id: &str, today: &str) -> bool {
        let state = self.ensure_date(profile_id, today);
        let had_bonus = state.bonus_minutes > 0;
        state.bonus_minutes = 0;
        state.warned_5 = false;
        state.warned_1 = false;
        had_bonus
    }

    pub fn warning_to_emit(
        &mut self,
        profile_id: &str,
        daily_limit_minutes: Option<u32>,
        today: &str,
    ) -> Option<TimeWarning> {
        let Some(limit) = daily_limit_minutes else {
            self.ensure_date(profile_id, today).warned_5 = false;
            self.ensure_date(profile_id, today).warned_1 = false;
            return None;
        };
        let status = self.status(profile_id, Some(limit), today);
        let state = self.ensure_date(profile_id, today);
        if status.remaining_seconds > 300 {
            state.warned_5 = false;
            state.warned_1 = false;
            return None;
        }
        if status.remaining_seconds > 60 {
            if !state.warned_5 {
                state.warned_5 = true;
                return Some(TimeWarning::FiveMinutes);
            }
            return None;
        }
        if status.remaining_seconds > 0 {
            if !state.warned_1 {
                state.warned_1 = true;
                return Some(TimeWarning::OneMinute);
            }
            return None;
        }
        if !state.warned_1 {
            state.warned_1 = true;
            return Some(TimeWarning::Exhausted);
        }
        None
    }

    pub fn profile_state(&self, profile_id: &str) -> Option<&ProfileTime> {
        self.profiles.get(profile_id)
    }

    pub fn remove_profile(&mut self, profile_id: &str) -> bool {
        self.profiles.remove(profile_id).is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeStatus {
    pub limited: bool,
    pub used_seconds: u64,
    pub bonus_minutes: u32,
    pub remaining_seconds: u64,
    pub remaining_minutes: u64,
    pub exhausted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeWarning {
    FiveMinutes,
    OneMinute,
    Exhausted,
}

pub enum TimekeepingLoad {
    Missing,
    Loaded(Timekeeping),
    Corrupt {
        message: String,
        backup: Option<PathBuf>,
    },
}

pub fn today_string() -> String {
    Local::now().date_naive().format("%Y-%m-%d").to_string()
}

pub fn load(paths: &XdgPaths) -> TimekeepingLoad {
    let path = paths.timekeeping_path();
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return TimekeepingLoad::Missing,
        Err(err) => {
            return TimekeepingLoad::Corrupt {
                message: format!("Zeitkontingent-Datei konnte nicht gelesen werden: {err}"),
                backup: None,
            }
        }
    };
    let mut timekeeping: Timekeeping = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(err) => {
            let backup = crate::storage::backup_file(&path).ok();
            return TimekeepingLoad::Corrupt {
                message: format!(
                    "timekeeping.json ist kein gültiges JSON ({err}). Die Datei wurde nicht überschrieben."
                ),
                backup,
            };
        }
    };
    if timekeeping.schema_version > SCHEMA_VERSION {
        let backup = crate::storage::backup_file(&path).ok();
        return TimekeepingLoad::Corrupt {
            message: format!(
                "timekeeping.json verwendet Schema {}, CouchDeck unterstützt höchstens {}.",
                timekeeping.schema_version, SCHEMA_VERSION
            ),
            backup,
        };
    }
    if timekeeping.schema_version < SCHEMA_VERSION {
        let _ = crate::storage::backup_file(&path);
        timekeeping.schema_version = SCHEMA_VERSION;
    }
    TimekeepingLoad::Loaded(timekeeping)
}

pub fn save(paths: &XdgPaths, timekeeping: &Timekeeping) -> Result<(), String> {
    let path = paths.timekeeping_path();
    let json = serde_json::to_vec_pretty(timekeeping)
        .map_err(|err| format!("Zeitkontingent konnte nicht serialisiert werden: {err}"))?;
    crate::storage::atomic_write(&path, &json, false).map_err(|err| {
        format!(
            "Zeitkontingent konnte nicht gespeichert werden ({}): {err}",
            path.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: &str = "2026-11-06";

    #[test]
    fn extra_minutes_add_up_and_are_capped_by_usage() {
        let mut timekeeping = Timekeeping::new();
        let status = timekeeping.add_usage("child", 700, Some(10), DAY);
        assert_eq!(status.used_seconds, 600);
        assert!(status.exhausted);
        // Usage stops at the current total once exhausted.
        let status = timekeeping.add_usage("child", 100, Some(10), DAY);
        assert_eq!(status.used_seconds, 600);
        timekeeping.add_extra_minutes("child", 5, DAY);
        timekeeping.add_extra_minutes("child", 10, DAY);
        let status = timekeeping.status("child", Some(10), DAY);
        assert_eq!(status.bonus_minutes, 15);
        assert_eq!(status.remaining_seconds, 15 * 60);
        let status = timekeeping.add_usage("child", 20 * 60, Some(10), DAY);
        assert!(status.exhausted);
        assert_eq!(status.used_seconds, 25 * 60);
    }

    #[test]
    fn day_rollover_resets_usage_and_bonus() {
        let mut timekeeping = Timekeeping::new();
        timekeeping.add_usage("child", 300, Some(10), DAY);
        timekeeping.add_extra_minutes("child", 15, DAY);
        let next = "2026-11-07";
        let status = timekeeping.status("child", Some(10), next);
        assert_eq!(status.used_seconds, 0);
        assert_eq!(status.bonus_minutes, 0);
        assert_eq!(status.remaining_seconds, 600);
    }

    #[test]
    fn reset_extra_minutes_keeps_usage() {
        let mut timekeeping = Timekeeping::new();
        timekeeping.add_usage("child", 120, Some(10), DAY);
        timekeeping.add_extra_minutes("child", 15, DAY);
        assert!(timekeeping.reset_extra_minutes("child", DAY));
        let status = timekeeping.status("child", Some(10), DAY);
        assert_eq!(status.bonus_minutes, 0);
        assert_eq!(status.used_seconds, 120);
        assert_eq!(status.remaining_seconds, 480);
    }

    #[test]
    fn warning_flags_survive_and_rearm_with_extra_time() {
        let mut timekeeping = Timekeeping::new();
        timekeeping.add_usage("child", 350, Some(10), DAY);
        assert_eq!(
            timekeeping.warning_to_emit("child", Some(10), DAY),
            Some(TimeWarning::FiveMinutes)
        );
        assert_eq!(timekeeping.warning_to_emit("child", Some(10), DAY), None);
        timekeeping.add_extra_minutes("child", 5, DAY);
        // Above the threshold again: no warning and flags re-arm.
        assert_eq!(timekeeping.warning_to_emit("child", Some(10), DAY), None);
        timekeeping.add_usage("child", 300, Some(10), DAY);
        assert_eq!(
            timekeeping.warning_to_emit("child", Some(10), DAY),
            Some(TimeWarning::FiveMinutes)
        );
    }

    #[test]
    fn save_load_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = XdgPaths::for_home(tmp.path());
        let mut timekeeping = Timekeeping::new();
        timekeeping.add_extra_minutes("child", 30, DAY);
        save(&paths, &timekeeping).unwrap();
        match load(&paths) {
            TimekeepingLoad::Loaded(loaded) => {
                assert_eq!(loaded.profile_state("child").unwrap().bonus_minutes, 30);
            }
            _ => panic!("expected loaded timekeeping"),
        }
    }

    #[test]
    fn corrupt_timekeeping_is_backed_up() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = XdgPaths::for_home(tmp.path());
        fs::create_dir_all(paths.data_dir()).unwrap();
        fs::write(paths.timekeeping_path(), b"{ broken").unwrap();
        match load(&paths) {
            TimekeepingLoad::Corrupt { backup, .. } => {
                assert!(backup.is_some_and(|path| path.exists()));
            }
            _ => panic!("expected corrupt timekeeping"),
        }
    }
}
