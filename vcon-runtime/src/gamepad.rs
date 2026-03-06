use std::fs;
use std::path::{Path, PathBuf};

use vcon_engine::{map_gamepad_state, InputFrame, InputProfile, RawGamepadState};

use crate::python_host::InputProvider;

const DEFAULT_STATE_FILE: &str = "/tmp/vcon-gamepad-input.txt";
const DEFAULT_DEBOUNCE_FRAMES: u8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControllerBackendKind {
    Scripted,
    File,
    OsNative,
}

#[derive(Debug, Clone)]
struct ControllerSample {
    connected: bool,
    profile: InputProfile,
    raw: RawGamepadState,
}

trait ControllerBackend {
    fn poll(&mut self, frame_idx: u32, fallback: &ControllerSample) -> ControllerSample;
}

struct FileControllerBackend {
    state_file: PathBuf,
}

impl FileControllerBackend {
    fn new(state_file: PathBuf) -> Self {
        Self { state_file }
    }
}

impl ControllerBackend for FileControllerBackend {
    fn poll(&mut self, _frame_idx: u32, fallback: &ControllerSample) -> ControllerSample {
        if let Ok(text) = fs::read_to_string(&self.state_file) {
            parse_state_file(&text, fallback)
        } else {
            fallback.clone()
        }
    }
}

struct ScriptedControllerBackend;

impl ControllerBackend for ScriptedControllerBackend {
    fn poll(&mut self, frame_idx: u32, fallback: &ControllerSample) -> ControllerSample {
        let stage = frame_idx % 120;
        let connected = stage < 30 || stage >= 60;
        let profile = if stage < 60 {
            InputProfile::Desktop
        } else {
            InputProfile::SteamDeck
        };

        if !connected {
            return ControllerSample {
                connected,
                profile,
                raw: RawGamepadState::default(),
            };
        }

        let phase = (stage as f64) / 120.0;
        let dpad_right = stage % 20 < 10;
        let dpad_up = stage % 40 < 20;

        let mut raw = fallback.raw.clone();
        raw.left_x = (phase * std::f64::consts::TAU).sin().clamp(-1.0, 1.0);
        raw.left_y = (phase * std::f64::consts::TAU).cos().clamp(-1.0, 1.0);
        raw.right_x = ((phase * 2.0) * std::f64::consts::TAU).sin().clamp(-1.0, 1.0);
        raw.right_y = ((phase * 2.0) * std::f64::consts::TAU).cos().clamp(-1.0, 1.0);
        raw.dpad_right = dpad_right;
        raw.dpad_left = !dpad_right;
        raw.dpad_up = dpad_up;
        raw.dpad_down = !dpad_up;
        raw.south = stage % 15 == 0;
        raw.east = stage % 22 == 0;
        raw.west = stage % 30 < 8;
        raw.north = stage % 27 < 5;
        raw.l1 = stage % 18 < 9;
        raw.r1 = stage % 24 < 12;
        raw.l2 = if stage % 16 < 8 { 0.75 } else { 0.2 };
        raw.r2 = if stage % 14 < 7 { 0.8 } else { 0.15 };
        raw.start = stage == 0 || stage == 60;
        raw.select = stage % 33 == 0;

        ControllerSample {
            connected,
            profile,
            raw,
        }
    }
}

struct OsNativeControllerBackend;

impl ControllerBackend for OsNativeControllerBackend {
    fn poll(&mut self, _frame_idx: u32, fallback: &ControllerSample) -> ControllerSample {
        // Placeholder backend: runtime stays disconnected unless explicitly configured.
        ControllerSample {
            connected: false,
            profile: fallback.profile,
            raw: RawGamepadState::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectionState {
    Connected,
    Disconnected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectionEvent {
    Connected,
    Disconnected,
    Reconnected,
}

pub struct GamepadInputProvider {
    backend: Box<dyn ControllerBackend>,
    sample: ControllerSample,
    connection_state: ConnectionState,
    pending_state: Option<(ConnectionState, u8)>,
    debounce_frames: u8,
    has_connected_once: bool,
}

impl GamepadInputProvider {
    pub fn new() -> Self {
        let backend_kind = detect_backend_kind();
        let state_file = std::env::var_os("VCON_GAMEPAD_STATE_FILE")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_STATE_FILE));
        Self::with_backend(backend_kind, state_file)
    }

    pub fn with_backend(backend_kind: ControllerBackendKind, state_file: PathBuf) -> Self {
        let backend = build_backend(backend_kind, state_file);

        Self {
            backend,
            sample: ControllerSample {
                connected: false,
                profile: InputProfile::SteamDeck,
                raw: RawGamepadState::default(),
            },
            connection_state: ConnectionState::Disconnected,
            pending_state: None,
            debounce_frames: DEFAULT_DEBOUNCE_FRAMES,
            has_connected_once: false,
        }
    }

    #[cfg(test)]
    fn with_debounce_frames(mut self, frames: u8) -> Self {
        self.debounce_frames = frames.max(1);
        self
    }

    fn connection_transition(
        &mut self,
        sampled_connected: bool,
    ) -> (ConnectionState, Option<ConnectionEvent>) {
        let observed = if sampled_connected {
            ConnectionState::Connected
        } else {
            ConnectionState::Disconnected
        };

        if observed == self.connection_state {
            self.pending_state = None;
            return (self.connection_state, None);
        }

        let mut stable_count = 1;
        if let Some((pending, count)) = self.pending_state {
            if pending == observed {
                stable_count = count.saturating_add(1);
            }
        }

        if stable_count < self.debounce_frames {
            self.pending_state = Some((observed, stable_count));
            return (self.connection_state, None);
        }

        self.pending_state = None;
        self.connection_state = observed;

        let event = match observed {
            ConnectionState::Connected => {
                if self.has_connected_once {
                    Some(ConnectionEvent::Reconnected)
                } else {
                    self.has_connected_once = true;
                    Some(ConnectionEvent::Connected)
                }
            }
            ConnectionState::Disconnected => Some(ConnectionEvent::Disconnected),
        };

        (self.connection_state, event)
    }
}

impl InputProvider for GamepadInputProvider {
    fn next_frame(&mut self, frame_idx: u32) -> InputFrame {
        self.sample = self.backend.poll(frame_idx, &self.sample);

        let (state, event) = self.connection_transition(self.sample.connected);
        if state == ConnectionState::Disconnected {
            self.sample.raw = RawGamepadState::default();
        }

        let mut frame = if state == ConnectionState::Connected {
            map_gamepad_state(self.sample.profile, &self.sample.raw)
        } else {
            InputFrame::default()
        };

        frame.set_action("ControllerConnectedState", state == ConnectionState::Connected);
        frame.set_action("ControllerConnected", matches!(event, Some(ConnectionEvent::Connected)));
        frame.set_action(
            "ControllerDisconnected",
            matches!(event, Some(ConnectionEvent::Disconnected)),
        );
        frame.set_action(
            "ControllerReconnected",
            matches!(event, Some(ConnectionEvent::Reconnected)),
        );

        frame
    }
}

fn detect_backend_kind() -> ControllerBackendKind {
    match std::env::var("VCON_CONTROLLER_BACKEND") {
        Ok(value) if value.eq_ignore_ascii_case("scripted") => ControllerBackendKind::Scripted,
        Ok(value) if value.eq_ignore_ascii_case("os-native") => ControllerBackendKind::OsNative,
        _ => ControllerBackendKind::File,
    }
}

fn build_backend(backend_kind: ControllerBackendKind, state_file: PathBuf) -> Box<dyn ControllerBackend> {
    match backend_kind {
        ControllerBackendKind::Scripted => Box::new(ScriptedControllerBackend),
        ControllerBackendKind::File => Box::new(FileControllerBackend::new(state_file)),
        ControllerBackendKind::OsNative => Box::new(OsNativeControllerBackend),
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "down" | "yes" | "on" => Some(true),
        "0" | "false" | "up" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn parse_profile(value: &str) -> Option<InputProfile> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("desktop") {
        Some(InputProfile::Desktop)
    } else if value.eq_ignore_ascii_case("steamdeck") || value.eq_ignore_ascii_case("steam_deck") {
        Some(InputProfile::SteamDeck)
    } else {
        None
    }
}

fn parse_state_file(input: &str, previous: &ControllerSample) -> ControllerSample {
    let mut next = previous.clone();
    let mut explicit_l2 = false;
    let mut explicit_r2 = false;

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.splitn(2, '=');
        let key = parts.next().unwrap_or("").trim();
        let value = parts.next().unwrap_or("").trim();
        if key.is_empty() {
            continue;
        }

        match key {
            "profile" => {
                if let Some(profile) = parse_profile(value) {
                    next.profile = profile;
                }
            }
            "connected" => {
                if let Some(connected) = parse_bool(value) {
                    next.connected = connected;
                }
            }
            "move_x" | "left_x" => {
                if let Ok(v) = value.parse::<f64>() {
                    next.raw.left_x = v;
                }
            }
            "move_y" | "left_y" => {
                if let Ok(v) = value.parse::<f64>() {
                    next.raw.left_y = v;
                }
            }
            "look_x" | "right_x" => {
                if let Ok(v) = value.parse::<f64>() {
                    next.raw.right_x = v;
                }
            }
            "look_y" | "right_y" => {
                if let Ok(v) = value.parse::<f64>() {
                    next.raw.right_y = v;
                }
            }
            "trigger_l" | "L2_axis" => {
                if let Ok(v) = value.parse::<f64>() {
                    next.raw.l2 = v;
                    explicit_l2 = true;
                }
            }
            "trigger_r" | "R2_axis" => {
                if let Ok(v) = value.parse::<f64>() {
                    next.raw.r2 = v;
                    explicit_r2 = true;
                }
            }
            "dpad_up" | "DPadUp" => {
                if let Some(v) = parse_bool(value) {
                    next.raw.dpad_up = v;
                }
            }
            "dpad_down" | "DPadDown" => {
                if let Some(v) = parse_bool(value) {
                    next.raw.dpad_down = v;
                }
            }
            "dpad_left" | "DPadLeft" => {
                if let Some(v) = parse_bool(value) {
                    next.raw.dpad_left = v;
                }
            }
            "dpad_right" | "DPadRight" => {
                if let Some(v) = parse_bool(value) {
                    next.raw.dpad_right = v;
                }
            }
            "A" => {
                if let Some(v) = parse_bool(value) {
                    next.raw.south = v;
                }
            }
            "B" => {
                if let Some(v) = parse_bool(value) {
                    next.raw.east = v;
                }
            }
            "X" => {
                if let Some(v) = parse_bool(value) {
                    next.raw.west = v;
                }
            }
            "Y" => {
                if let Some(v) = parse_bool(value) {
                    next.raw.north = v;
                }
            }
            "L1" => {
                if let Some(v) = parse_bool(value) {
                    next.raw.l1 = v;
                }
            }
            "R1" => {
                if let Some(v) = parse_bool(value) {
                    next.raw.r1 = v;
                }
            }
            "L2" => {
                if let Some(v) = parse_bool(value) {
                    if !explicit_l2 {
                        next.raw.l2 = if v { 1.0 } else { 0.0 };
                    }
                }
            }
            "R2" => {
                if let Some(v) = parse_bool(value) {
                    if !explicit_r2 {
                        next.raw.r2 = if v { 1.0 } else { 0.0 };
                    }
                }
            }
            "Start" => {
                if let Some(v) = parse_bool(value) {
                    next.raw.start = v;
                }
            }
            "Select" => {
                if let Some(v) = parse_bool(value) {
                    next.raw.select = v;
                }
            }
            _ => {}
        }
    }

    next
}

#[cfg(test)]
mod tests {
    use super::{
        parse_state_file, ConnectionEvent, ConnectionState, ControllerBackendKind, ControllerSample,
        GamepadInputProvider,
    };
    use crate::python_host::InputProvider;
    use vcon_engine::{InputProfile, RawGamepadState};

    #[test]
    fn parses_full_state_file_mapping() {
        let previous = ControllerSample {
            connected: false,
            profile: InputProfile::SteamDeck,
            raw: RawGamepadState::default(),
        };
        let input = "connected=true\nprofile=desktop\nmove_x=0.4\nmove_y=-0.2\nlook_x=-0.7\nlook_y=0.5\ndpad_up=true\ndpad_left=1\nA=down\nB=true\nX=false\nY=1\nL1=true\nR1=false\nL2=down\nR2_axis=0.75\nStart=false\nSelect=true\n";
        let state = parse_state_file(input, &previous);

        assert!(state.connected);
        assert_eq!(state.profile, InputProfile::Desktop);
        assert_eq!(state.raw.left_x, 0.4);
        assert_eq!(state.raw.left_y, -0.2);
        assert_eq!(state.raw.right_x, -0.7);
        assert_eq!(state.raw.right_y, 0.5);
        assert!(state.raw.dpad_up);
        assert!(state.raw.dpad_left);
        assert!(state.raw.south);
        assert!(state.raw.east);
        assert!(!state.raw.west);
        assert!(state.raw.north);
        assert!(state.raw.l1);
        assert!(!state.raw.r1);
        assert_eq!(state.raw.l2, 1.0);
        assert_eq!(state.raw.r2, 0.75);
        assert!(!state.raw.start);
        assert!(state.raw.select);
    }

    #[test]
    fn transition_debounces_and_emits_reconnect() {
        let mut provider = GamepadInputProvider::with_backend(
            ControllerBackendKind::Scripted,
            std::path::PathBuf::new(),
        )
        .with_debounce_frames(2);

        let f0 = provider.next_frame(0);
        assert!(!f0.action_pressed("ControllerConnected"));
        assert!(!f0.action_pressed("ControllerConnectedState"));

        let f1 = provider.next_frame(1);
        assert!(f1.action_pressed("ControllerConnected"));
        assert!(f1.action_pressed("ControllerConnectedState"));

        let f30 = provider.next_frame(30);
        assert!(f30.action_pressed("ControllerConnectedState"));
        let f31 = provider.next_frame(31);
        assert!(f31.action_pressed("ControllerDisconnected"));
        assert!(!f31.action_pressed("ControllerConnectedState"));

        let f60 = provider.next_frame(60);
        assert!(!f60.action_pressed("ControllerConnectedState"));
        let f61 = provider.next_frame(61);
        assert!(f61.action_pressed("ControllerReconnected"));
        assert!(f61.action_pressed("ControllerConnectedState"));
    }

    #[test]
    fn disconnect_resets_stale_state() {
        let mut provider = GamepadInputProvider::with_backend(
            ControllerBackendKind::Scripted,
            std::path::PathBuf::new(),
        )
        .with_debounce_frames(1);

        let connected = provider.next_frame(0);
        assert!(connected.axis("move_x") != 0.0 || connected.axis("move_y") != 0.0);

        let disconnected = provider.next_frame(30);
        assert_eq!(disconnected.axis("move_x"), 0.0);
        assert_eq!(disconnected.axis("move_y"), 0.0);
        assert!(!disconnected.action_pressed("A"));
        assert!(disconnected.action_pressed("ControllerDisconnected"));
    }

    #[test]
    fn connection_transition_reports_expected_events() {
        let mut provider = GamepadInputProvider::with_backend(
            ControllerBackendKind::File,
            std::path::PathBuf::new(),
        )
        .with_debounce_frames(1);

        let (state, event) = provider.connection_transition(true);
        assert_eq!(state, ConnectionState::Connected);
        assert_eq!(event, Some(ConnectionEvent::Connected));

        let (state, event) = provider.connection_transition(false);
        assert_eq!(state, ConnectionState::Disconnected);
        assert_eq!(event, Some(ConnectionEvent::Disconnected));

        let (_state, _event) = provider.connection_transition(true);
        let (state, event) = provider.connection_transition(false);
        assert_eq!(state, ConnectionState::Disconnected);
        assert_eq!(event, Some(ConnectionEvent::Disconnected));
    }
}

#[allow(dead_code)]
fn _state_file_exists(path: &Path) -> bool {
    path.exists()
}
