use crate::input::InputFrame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputProfile {
    Desktop,
    SteamDeck,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RawGamepadState {
    pub left_x: f64,
    pub left_y: f64,
    pub south: bool,
    pub start: bool,
}

pub fn map_gamepad_state(profile: InputProfile, raw: &RawGamepadState) -> InputFrame {
    let deadzone = match profile {
        InputProfile::Desktop => 0.10,
        InputProfile::SteamDeck => 0.08,
    };

    let mut frame = InputFrame::default();
    frame.set_axis("move_x", apply_deadzone(raw.left_x, deadzone));
    frame.set_axis("move_y", apply_deadzone(-raw.left_y, deadzone));
    frame.set_action("A", raw.south);
    frame.set_action("Start", raw.start);

    frame
}

fn apply_deadzone(value: f64, dz: f64) -> f64 {
    if value.abs() < dz {
        0.0
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::{map_gamepad_state, InputProfile, RawGamepadState};

    #[test]
    fn desktop_profile_applies_deadzone() {
        let raw = RawGamepadState {
            left_x: 0.05,
            left_y: -0.25,
            south: true,
            start: false,
        };
        let mapped = map_gamepad_state(InputProfile::Desktop, &raw);

        assert_eq!(mapped.axis("move_x"), 0.0);
        assert_eq!(mapped.axis("move_y"), 0.25);
        assert!(mapped.action_pressed("A"));
    }

    #[test]
    fn steam_deck_profile_has_lower_deadzone() {
        let raw = RawGamepadState {
            left_x: 0.09,
            left_y: 0.0,
            south: false,
            start: true,
        };
        let mapped = map_gamepad_state(InputProfile::SteamDeck, &raw);

        assert_eq!(mapped.axis("move_x"), 0.09);
        assert!(mapped.action_pressed("Start"));
    }
}
