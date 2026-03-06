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
    pub right_x: f64,
    pub right_y: f64,
    pub dpad_up: bool,
    pub dpad_down: bool,
    pub dpad_left: bool,
    pub dpad_right: bool,
    pub south: bool,
    pub east: bool,
    pub west: bool,
    pub north: bool,
    pub l1: bool,
    pub r1: bool,
    pub l2: f64,
    pub r2: f64,
    pub start: bool,
    pub select: bool,
}

pub fn map_gamepad_state(profile: InputProfile, raw: &RawGamepadState) -> InputFrame {
    let deadzone = match profile {
        InputProfile::Desktop => 0.10,
        InputProfile::SteamDeck => 0.08,
    };

    let mut frame = InputFrame::default();
    frame.set_axis("move_x", apply_deadzone(raw.left_x, deadzone));
    frame.set_axis("move_y", apply_deadzone(-raw.left_y, deadzone));
    frame.set_axis("look_x", apply_deadzone(raw.right_x, deadzone));
    frame.set_axis("look_y", apply_deadzone(-raw.right_y, deadzone));
    frame.set_axis("dpad_x", dpad_axis(raw.dpad_right, raw.dpad_left));
    frame.set_axis("dpad_y", dpad_axis(raw.dpad_up, raw.dpad_down));
    frame.set_axis("trigger_l", apply_deadzone(raw.l2, deadzone));
    frame.set_axis("trigger_r", apply_deadzone(raw.r2, deadzone));
    frame.set_action("A", raw.south);
    frame.set_action("B", raw.east);
    frame.set_action("X", raw.west);
    frame.set_action("Y", raw.north);
    frame.set_action("L1", raw.l1);
    frame.set_action("R1", raw.r1);
    frame.set_action("L2", raw.l2 > 0.5);
    frame.set_action("R2", raw.r2 > 0.5);
    frame.set_action("DPadUp", raw.dpad_up);
    frame.set_action("DPadDown", raw.dpad_down);
    frame.set_action("DPadLeft", raw.dpad_left);
    frame.set_action("DPadRight", raw.dpad_right);
    frame.set_action("Start", raw.start);
    frame.set_action("Select", raw.select);

    frame
}

fn apply_deadzone(value: f64, dz: f64) -> f64 {
    if value.abs() < dz {
        0.0
    } else {
        value
    }
}

fn dpad_axis(positive: bool, negative: bool) -> f64 {
    match (positive, negative) {
        (true, false) => 1.0,
        (false, true) => -1.0,
        _ => 0.0,
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
            right_x: 0.01,
            right_y: -0.8,
            dpad_up: true,
            dpad_down: false,
            dpad_left: true,
            dpad_right: false,
            south: true,
            east: false,
            west: true,
            north: false,
            l1: true,
            r1: false,
            l2: 0.75,
            r2: 0.45,
            start: false,
            select: true,
        };
        let mapped = map_gamepad_state(InputProfile::Desktop, &raw);

        assert_eq!(mapped.axis("move_x"), 0.0);
        assert_eq!(mapped.axis("move_y"), 0.25);
        assert_eq!(mapped.axis("look_x"), 0.0);
        assert_eq!(mapped.axis("look_y"), 0.8);
        assert_eq!(mapped.axis("dpad_x"), -1.0);
        assert_eq!(mapped.axis("dpad_y"), 1.0);
        assert_eq!(mapped.axis("trigger_l"), 0.75);
        assert_eq!(mapped.axis("trigger_r"), 0.45);
        assert!(mapped.action_pressed("A"));
        assert!(mapped.action_pressed("X"));
        assert!(mapped.action_pressed("L1"));
        assert!(mapped.action_pressed("L2"));
        assert!(mapped.action_pressed("DPadUp"));
        assert!(mapped.action_pressed("DPadLeft"));
        assert!(mapped.action_pressed("Select"));
        assert!(!mapped.action_pressed("B"));
        assert!(!mapped.action_pressed("R2"));
    }

    #[test]
    fn steam_deck_profile_has_lower_deadzone() {
        let raw = RawGamepadState {
            left_x: 0.09,
            left_y: 0.0,
            right_x: -0.09,
            right_y: 0.0,
            l2: 0.51,
            south: false,
            start: true,
            ..RawGamepadState::default()
        };
        let mapped = map_gamepad_state(InputProfile::SteamDeck, &raw);

        assert_eq!(mapped.axis("move_x"), 0.09);
        assert_eq!(mapped.axis("look_x"), -0.09);
        assert!(mapped.action_pressed("L2"));
        assert!(mapped.action_pressed("Start"));
    }
}
