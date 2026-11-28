//! Gamepad input normalisation via `gilrs`.
//!
//! The controller lives on a background thread. It polls `gilrs` (which is how
//! Linux exposes evdev) at 10 ms intervals and wakes egui only when an event is
//! produced, so the UI can stay idle without a 60 fps repaint loop.
//!
//! Semantic mapping:
//! * D-pad / left stick: focus navigation
//! * South (Xbox A / PS Cross): confirm
//! * East (Xbox B / PS Circle): back
//! * Start: menu
//! * LB/RB (shoulder buttons): previous/next profile (the UI displays this
//!   only in the profile view)

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use eframe::egui;
use gilrs::{Axis, Button, EventType, GamepadId, Gilrs};

use crate::logging::{log_info, log_warn};
use crate::model::ControllerHomeChord;
use crate::paths::XdgPaths;
use crate::session::{self, DesktopEnvironment};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavAction {
    Up,
    Down,
    Left,
    Right,
    Confirm,
    Back,
    Menu,
    PrevProfile,
    NextProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeAction {
    /// Return to the already-running launcher. Future actions can be added
    /// here (for example an explicit end-of-session action) without changing
    /// the chord detector.
    ActivateCapyDeck,
}

#[derive(Debug, Clone)]
pub enum ControllerEvent {
    Nav(NavAction),
    Connected(String),
    Disconnected(String),
    Error(String),
    /// Emitted once when the background thread has initialised.
    Ready(usize),
    /// The configured global controller-home chord was held long enough.
    HomeAction(HomeAction),
}

enum ControllerCommand {
    /// The CapyDeck window gained or lost input focus. When inactive, all
    /// navigation events are suppressed and held-button/stick state is reset so
    /// a game running in the foreground cannot keep navigating the launcher.
    SetActive(bool),
    SetHomeConfig {
        enabled: bool,
        chord: ControllerHomeChord,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct HomeConfig {
    pub enabled: bool,
    pub chord: ControllerHomeChord,
}

pub struct ControllerHandle {
    rx: Receiver<ControllerEvent>,
    commands: Sender<ControllerCommand>,
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl ControllerHandle {
    pub fn drain(&self) -> Vec<ControllerEvent> {
        self.rx.try_iter().collect()
    }

    /// Enable/disable navigation processing without destroying hotplug
    /// tracking. The controller thread still reports connect/disconnect events
    /// while inactive.
    pub fn set_active(&self, active: bool) {
        let _ = self.commands.send(ControllerCommand::SetActive(active));
    }

    pub fn set_home_config(&self, enabled: bool, chord: ControllerHomeChord) {
        let _ = self
            .commands
            .send(ControllerCommand::SetHomeConfig { enabled, chord });
    }
}

impl Drop for ControllerHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

/// Start the controller thread. When `enabled` is false an empty handle is
/// returned (useful for `--no-controller` debugging).
pub fn spawn(
    ctx: egui::Context,
    enabled: bool,
    home: HomeConfig,
    paths: XdgPaths,
    focus_state: Arc<AtomicBool>,
) -> ControllerHandle {
    let (tx, rx) = mpsc::channel();
    let (command_tx, command_rx) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    if !enabled {
        return ControllerHandle {
            rx,
            commands: command_tx,
            stop,
            join: None,
        };
    }
    let stop_thread = stop.clone();
    let join = thread::Builder::new()
        .name("capydeck-gamepad".to_owned())
        .spawn(move || run(ctx, tx, command_rx, stop_thread, home, paths, focus_state))
        .ok();
    ControllerHandle {
        rx,
        commands: command_tx,
        stop,
        join,
    }
}

fn run(
    ctx: egui::Context,
    tx: mpsc::Sender<ControllerEvent>,
    commands: Receiver<ControllerCommand>,
    stop: Arc<AtomicBool>,
    mut home: HomeConfig,
    paths: XdgPaths,
    focus_state: Arc<AtomicBool>,
) {
    let mut gilrs = match Gilrs::new() {
        Ok(gilrs) => gilrs,
        Err(err) => {
            let _ = tx.send(ControllerEvent::Error(format!(
                "Gamepad-Unterstützung konnte nicht initialisiert werden: {err}. Tastatur und Maus funktionieren weiterhin."
            )));
            ctx.request_repaint();
            return;
        }
    };

    let mut states: HashMap<GamepadId, PadState> = HashMap::new();
    let mut home_detectors: HashMap<GamepadId, HomeDetector> = HashMap::new();
    let mut names: HashMap<GamepadId, String> = HashMap::new();
    if home.enabled {
        log_info(
            "controller",
            format!("Controller-Home aktiv: {:?} (2 s halten).", home.chord),
        );
    }
    let mut initial: Vec<(GamepadId, String)> = gilrs
        .gamepads()
        .map(|(id, gamepad)| (id, gamepad.name().to_owned()))
        .collect();
    initial.sort_by(|a, b| a.1.cmp(&b.1));
    for (id, name) in &initial {
        states.insert(*id, PadState::default());
        home_detectors.insert(*id, HomeDetector::default());
        names.insert(*id, name.clone());
        let _ = tx.send(ControllerEvent::Connected(name.clone()));
    }
    let _ = tx.send(ControllerEvent::Ready(initial.len()));
    ctx.request_repaint();

    let mut active = true;
    let mut last_tick = Instant::now();
    while !stop.load(Ordering::Relaxed) {
        while let Ok(command) = commands.try_recv() {
            match command {
                ControllerCommand::SetHomeConfig { enabled, chord } => {
                    home = HomeConfig { enabled, chord };
                    for detector in home_detectors.values_mut() {
                        *detector = HomeDetector::default();
                    }
                    log_info(
                        "controller",
                        format!("Controller-Home-Konfiguration geändert: enabled={enabled}, chord={chord:?}."),
                    );
                }
                ControllerCommand::SetActive(new_active) => {
                    if new_active != active {
                        active = new_active;
                        // Never carry a held direction or stick deflection
                        // across a focus transition.
                        for state in states.values_mut() {
                            *state = PadState::default();
                        }
                    }
                }
            }
        }

        let mut produced = false;
        while let Some(event) = gilrs.next_event() {
            match event.event {
                EventType::Connected => {
                    if let Some(gamepad) = gilrs.connected_gamepad(event.id) {
                        states.insert(event.id, PadState::default());
                        home_detectors.insert(event.id, HomeDetector::default());
                        let name = gamepad.name().to_owned();
                        names.insert(event.id, name.clone());
                        let _ = tx.send(ControllerEvent::Connected(name));
                        produced = true;
                    }
                }
                EventType::Disconnected => {
                    states.remove(&event.id);
                    home_detectors.remove(&event.id);
                    let name = names
                        .remove(&event.id)
                        .unwrap_or_else(|| "Unbekannter Controller".to_owned());
                    let _ = tx.send(ControllerEvent::Disconnected(name));
                    produced = true;
                }
                EventType::ButtonPressed(button, _) => {
                    if home.enabled {
                        if let Some(detector) = home_detectors.get_mut(&event.id) {
                            detector.button(button, true, Instant::now(), home.chord);
                        }
                    }
                    if active {
                        states.entry(event.id).or_default();
                        if let Some(state) = states.get_mut(&event.id) {
                            state.press(button);
                        }
                        // Directional D-pad input is emitted by the repeat logic
                        // below. Emitting it here as well would double-step.
                        if !is_dpad_button(button) {
                            if let Some(action) = button_action(button) {
                                let _ = tx.send(ControllerEvent::Nav(action));
                                produced = true;
                            }
                        }
                    }
                }
                EventType::ButtonReleased(button, _) => {
                    if home.enabled {
                        if let Some(detector) = home_detectors.get_mut(&event.id) {
                            detector.button(button, false, Instant::now(), home.chord);
                        }
                    }
                    if active {
                        if let Some(state) = states.get_mut(&event.id) {
                            state.release(button);
                        }
                    }
                }
                EventType::AxisChanged(axis, value, _) => {
                    if active {
                        if let Some(state) = states.get_mut(&event.id) {
                            state.set_axis(axis, value);
                        }
                    }
                }
                EventType::ButtonChanged(_, _, _)
                | EventType::ButtonRepeated(_, _)
                | EventType::Dropped
                | EventType::ForceFeedbackEffectCompleted => {}
                _ => {}
            }
        }

        let now = Instant::now();
        if home.enabled {
            let mut chord_ready = false;
            for detector in home_detectors.values_mut() {
                if detector.tick(now, home.chord) {
                    chord_ready = true;
                }
            }
            if chord_ready {
                log_info(
                    "controller",
                    "Controller-Home-Chord erkannt; Aktivierung wird angefordert.",
                );
                let _ = tx.send(ControllerEvent::HomeAction(HomeAction::ActivateCapyDeck));
                trigger_home_background(&paths, &focus_state);
                produced = true;
            }
        }
        let elapsed = now.saturating_duration_since(last_tick);
        if active && elapsed >= Duration::from_millis(8) {
            last_tick = now;
            for state in states.values_mut() {
                if let Some(action) = state.tick_repeat(now, now) {
                    let _ = tx.send(ControllerEvent::Nav(action));
                    produced = true;
                }
            }
        }

        if produced {
            ctx.request_repaint();
        }
        thread::sleep(Duration::from_millis(10));
    }
}

const HOME_HOLD: Duration = Duration::from_secs(2);

#[derive(Debug, Default)]
struct HomeDetector {
    held: HashSet<Button>,
    since: Option<Instant>,
    fired: bool,
}

impl HomeDetector {
    fn button(&mut self, button: Button, pressed: bool, now: Instant, chord: ControllerHomeChord) {
        if pressed {
            self.held.insert(button);
        } else {
            self.held.remove(&button);
        }
        let buttons = chord_buttons(chord);
        if buttons.iter().all(|button| self.held.contains(button)) {
            if self.since.is_none() {
                self.since = Some(now);
                self.fired = false;
            }
        } else {
            self.since = None;
            self.fired = false;
        }
    }

    fn tick(&mut self, now: Instant, chord: ControllerHomeChord) -> bool {
        let buttons = chord_buttons(chord);
        if !buttons.iter().all(|button| self.held.contains(button)) {
            return false;
        }
        let Some(since) = self.since else {
            self.since = Some(now);
            return false;
        };
        if !self.fired && now.saturating_duration_since(since) >= HOME_HOLD {
            self.fired = true;
            return true;
        }
        false
    }
}

fn chord_buttons(chord: ControllerHomeChord) -> [Button; 2] {
    match chord {
        ControllerHomeChord::StartSelect => [Button::Start, Button::Select],
        ControllerHomeChord::StartLeftStick => [Button::Start, Button::LeftThumb],
        ControllerHomeChord::StartRightStick => [Button::Start, Button::RightThumb],
    }
}

fn trigger_home_background(paths: &XdgPaths, focus_state: &Arc<AtomicBool>) {
    let session = session::detect_current();
    if session.environment == Some(DesktopEnvironment::Kde)
        && std::env::var_os("WAYLAND_DISPLAY").is_some()
        && !crate::kwin_activation::activate_after(
            paths.clone(),
            Duration::from_millis(50),
            focus_state.clone(),
        )
    {
        log_warn(
            "controller",
            "Controller-Home: KWin-Fallback läuft bereits; keine weitere Anforderung.",
        );
    }
}

fn is_dpad_button(button: Button) -> bool {
    matches!(
        button,
        Button::DPadUp | Button::DPadDown | Button::DPadLeft | Button::DPadRight
    )
}

fn button_action(button: Button) -> Option<NavAction> {
    match button {
        Button::South => Some(NavAction::Confirm),
        Button::East => Some(NavAction::Back),
        Button::Start => Some(NavAction::Menu),
        // gilrs maps BTN_LT/BTN_RT ("leftshoulder"/"rightshoulder") to
        // LeftTrigger/RightTrigger. The *2 variants are the analog LT/RT
        // triggers, not LB/RB.
        Button::LeftTrigger => Some(NavAction::PrevProfile),
        Button::RightTrigger => Some(NavAction::NextProfile),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dir {
    Up,
    Down,
    Left,
    Right,
}

impl Dir {
    fn action(self) -> NavAction {
        match self {
            Self::Up => NavAction::Up,
            Self::Down => NavAction::Down,
            Self::Left => NavAction::Left,
            Self::Right => NavAction::Right,
        }
    }
}

const INITIAL_REPEAT: Duration = Duration::from_millis(340);
const HELD_REPEAT: Duration = Duration::from_millis(115);
const STICK_ACTIVATE: f32 = 0.42;
const STICK_RELEASE: f32 = 0.25;

#[derive(Debug)]
struct PadState {
    dpad_stack: Vec<Dir>,
    dpad_x: f32,
    dpad_y: f32,
    stick_x: f32,
    stick_y: f32,
    stick_dir: Option<Dir>,
    last_dir: Option<Dir>,
    next_repeat: Instant,
}

impl Default for PadState {
    fn default() -> Self {
        Self {
            dpad_stack: Vec::new(),
            dpad_x: 0.0,
            dpad_y: 0.0,
            stick_x: 0.0,
            stick_y: 0.0,
            stick_dir: None,
            last_dir: None,
            next_repeat: Instant::now(),
        }
    }
}

fn dpad_dir(button: Button) -> Option<Dir> {
    match button {
        Button::DPadUp => Some(Dir::Up),
        Button::DPadDown => Some(Dir::Down),
        Button::DPadLeft => Some(Dir::Left),
        Button::DPadRight => Some(Dir::Right),
        _ => None,
    }
}

impl PadState {
    fn press(&mut self, button: Button) {
        if let Some(dir) = dpad_dir(button) {
            // Most recently pressed direction wins, which makes diagonal
            // mis-presses deterministic. Releasing it returns to the still
            // held direction.
            self.dpad_stack.retain(|d| *d != dir);
            self.dpad_stack.push(dir);
        }
    }

    fn release(&mut self, button: Button) {
        if let Some(dir) = dpad_dir(button) {
            self.dpad_stack.retain(|d| *d != dir);
        }
    }

    fn set_axis(&mut self, axis: Axis, value: f32) {
        match axis {
            Axis::LeftStickX => self.stick_x = value.clamp(-1.0, 1.0),
            Axis::LeftStickY => self.stick_y = value.clamp(-1.0, 1.0),
            Axis::DPadX => self.dpad_x = value.clamp(-1.0, 1.0),
            Axis::DPadY => self.dpad_y = value.clamp(-1.0, 1.0),
            _ => {}
        }
        self.stick_dir = stick_direction(self.stick_x, self.stick_y, self.stick_dir);
    }

    fn dpad_axis_dir(&self) -> Option<Dir> {
        if self.dpad_x.abs() >= 0.5 {
            Some(if self.dpad_x < 0.0 {
                Dir::Left
            } else {
                Dir::Right
            })
        } else if self.dpad_y.abs() >= 0.5 {
            Some(if self.dpad_y < 0.0 {
                Dir::Up
            } else {
                Dir::Down
            })
        } else {
            None
        }
    }

    fn tick_repeat(&mut self, now: Instant, _started: Instant) -> Option<NavAction> {
        let dir = self
            .dpad_stack
            .last()
            .copied()
            .or_else(|| self.dpad_axis_dir())
            .or(self.stick_dir);
        if dir != self.last_dir {
            self.last_dir = dir;
            if let Some(dir) = dir {
                self.next_repeat = now + INITIAL_REPEAT;
                return Some(dir.action());
            }
            return None;
        }
        if let Some(dir) = dir {
            if now >= self.next_repeat {
                self.next_repeat = now + HELD_REPEAT;
                return Some(dir.action());
            }
        }
        None
    }
}

fn stick_direction(x: f32, y: f32, previous: Option<Dir>) -> Option<Dir> {
    let magnitude = (x * x + y * y).sqrt();
    if magnitude < STICK_RELEASE {
        return None;
    }
    if magnitude < STICK_ACTIVATE {
        return previous;
    }
    if x.abs() >= y.abs() {
        Some(if x < 0.0 { Dir::Left } else { Dir::Right })
    } else {
        // gilrs: positive Y is down.
        Some(if y < 0.0 { Dir::Up } else { Dir::Down })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn south_is_confirm_east_is_back() {
        assert_eq!(button_action(Button::South), Some(NavAction::Confirm));
        assert_eq!(button_action(Button::East), Some(NavAction::Back));
        assert_eq!(button_action(Button::Start), Some(NavAction::Menu));
        assert_eq!(
            button_action(Button::LeftTrigger),
            Some(NavAction::PrevProfile)
        );
        assert_eq!(
            button_action(Button::RightTrigger),
            Some(NavAction::NextProfile)
        );
        assert_eq!(button_action(Button::LeftTrigger2), None);
        assert_eq!(button_action(Button::RightTrigger2), None);
    }

    #[test]
    fn home_chord_requires_both_buttons_for_two_seconds() {
        let t = Instant::now();
        let mut detector = HomeDetector::default();
        detector.button(Button::Start, true, t, ControllerHomeChord::StartSelect);
        detector.button(
            Button::Select,
            true,
            t + Duration::from_millis(10),
            ControllerHomeChord::StartSelect,
        );
        assert!(!detector.tick(
            t + Duration::from_millis(2009),
            ControllerHomeChord::StartSelect
        ));
        assert!(detector.tick(
            t + Duration::from_millis(2010),
            ControllerHomeChord::StartSelect
        ));
        assert!(!detector.tick(
            t + Duration::from_millis(2500),
            ControllerHomeChord::StartSelect
        ));
        detector.button(
            Button::Select,
            false,
            t + Duration::from_millis(2600),
            ControllerHomeChord::StartSelect,
        );
        detector.button(
            Button::Select,
            true,
            t + Duration::from_millis(2700),
            ControllerHomeChord::StartSelect,
        );
        assert!(detector.tick(
            t + Duration::from_millis(4700),
            ControllerHomeChord::StartSelect
        ));
    }

    #[test]
    fn home_chord_alternatives_map_to_stick_clicks() {
        assert_eq!(
            chord_buttons(ControllerHomeChord::StartLeftStick),
            [Button::Start, Button::LeftThumb]
        );
        assert_eq!(
            chord_buttons(ControllerHomeChord::StartRightStick),
            [Button::Start, Button::RightThumb]
        );
    }

    #[test]
    fn stick_deadzone_and_hysteresis() {
        assert_eq!(stick_direction(0.1, 0.1, None), None);
        assert_eq!(stick_direction(0.62, 0.1, None), Some(Dir::Right));
        assert_eq!(
            stick_direction(0.30, 0.1, Some(Dir::Right)),
            Some(Dir::Right)
        );
        assert_eq!(stick_direction(0.20, 0.1, Some(Dir::Right)), None);
        assert_eq!(stick_direction(0.0, -0.8, None), Some(Dir::Up));
        assert_eq!(stick_direction(0.0, 0.8, None), Some(Dir::Down));
    }

    #[test]
    fn repeat_emits_initial_then_waits() {
        let mut state = PadState::default();
        state.press(Button::DPadRight);
        let t0 = Instant::now();
        assert_eq!(state.tick_repeat(t0, t0), Some(NavAction::Right));
        assert_eq!(state.tick_repeat(t0 + Duration::from_millis(100), t0), None);
        assert_eq!(
            state.tick_repeat(t0 + INITIAL_REPEAT, t0),
            Some(NavAction::Right)
        );
        assert_eq!(
            state.tick_repeat(t0 + INITIAL_REPEAT + HELD_REPEAT, t0),
            Some(NavAction::Right)
        );
        state.release(Button::DPadRight);
        assert_eq!(state.tick_repeat(t0 + INITIAL_REPEAT * 3, t0), None);
    }

    #[test]
    fn dpad_release_returns_to_stick() {
        let mut state = PadState::default();
        state.set_axis(Axis::LeftStickX, 0.9);
        state.press(Button::DPadDown);
        let t0 = Instant::now();
        assert_eq!(state.tick_repeat(t0, t0), Some(NavAction::Down));
        state.release(Button::DPadDown);
        assert_eq!(
            state.tick_repeat(t0 + Duration::from_millis(1), t0),
            Some(NavAction::Right)
        );
    }
}
