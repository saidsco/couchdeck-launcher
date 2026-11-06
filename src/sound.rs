//! Best-effort alert sound.
//!
//! CouchDeck does not bundle an audio engine. It tries common PipeWire/Pulse/
//! ALSA/Canberra players in the background. A missing player must never affect
//! time accounting or block the UI.

use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;

const FREEDESKTOP_BELL: &str = "/usr/share/sounds/freedesktop/stereo/bell.oga";
const ALSA_BELL: &str = "/usr/share/sounds/alsa/Front_Center.wav";

pub fn play_alert() {
    thread::spawn(|| {
        if try_canberra()
            || try_player("paplay", FREEDESKTOP_BELL)
            || try_player("pw-play", FREEDESKTOP_BELL)
            || try_player("aplay", ALSA_BELL)
        {
            return;
        }
        crate::logging::log_info(
            "sound",
            "Kein Audio-Player für den Zeitwarnungs-Ton gefunden (canberra/paplay/pw-play/aplay).",
        );
    });
}

fn try_canberra() -> bool {
    Command::new("canberra-gtk-play")
        .args(["-i", "bell"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn try_player(program: &str, sound: &str) -> bool {
    if !Path::new(sound).is_file() {
        return false;
    }
    Command::new(program)
        .arg(sound)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}
