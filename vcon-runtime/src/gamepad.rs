use std::fs;
use std::path::PathBuf;

use vcon_engine::{map_gamepad_state, InputFrame, InputProfile, RawGamepadState};

use crate::python_host::InputProvider;

pub struct GamepadInputProvider {
    state_file: PathBuf,
    profile: InputProfile,
    cached: InputFrame,
}

impl GamepadInputProvider {
    pub fn new() -> Self {
        Self {
            state_file: PathBuf::from("/tmp/vcon-gamepad-input.txt"),
            profile: InputProfile::SteamDeck,
            cached: InputFrame::default(),
        }
    }

    fn poll_state(&mut self) {
        if let Ok(text) = fs::read_to_string(&self.state_file) {
            self.cached = parse_state_file(&text, self.profile, self.cached.clone());
        }
    }
}

impl InputProvider for GamepadInputProvider {
    fn next_frame(&mut self, _frame_idx: u32) -> InputFrame {
        self.poll_state();
        self.cached.clone()
    }
}

fn parse_state_file(input: &str, profile: InputProfile, mut previous: InputFrame) -> InputFrame {
    let mut raw = RawGamepadState {
        left_x: previous.axis("move_x"),
        left_y: -previous.axis("move_y"),
        south: previous.action_pressed("A"),
        start: previous.action_pressed("Start"),
    };

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
            "move_x" => {
                if let Ok(v) = value.parse::<f64>() {
                    raw.left_x = v;
                }
            }
            "move_y" => {
                if let Ok(v) = value.parse::<f64>() {
                    raw.left_y = v;
                }
            }
            "A" => {
                let down = matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "down");
                raw.south = down;
            }
            "Start" => {
                let down = matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "down");
                raw.start = down;
            }
            _ => {}
        }
    }

    previous = map_gamepad_state(profile, &raw);
    previous
}

#[cfg(test)]
mod tests {
    use super::parse_state_file;
    use vcon_engine::{InputFrame, InputProfile};

    #[test]
    fn parses_axes_and_actions() {
        let input = "move_x=0.4\nmove_y=-0.2\nA=down\nStart=false\n";
        let frame = parse_state_file(input, InputProfile::SteamDeck, InputFrame::default());

        assert_eq!(frame.axis("move_x"), 0.4);
        assert_eq!(frame.axis("move_y"), 0.2);
        assert!(frame.action_pressed("A"));
        assert!(!frame.action_pressed("Start"));
    }
}
