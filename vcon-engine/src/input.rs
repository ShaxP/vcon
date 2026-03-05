use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq)]
pub struct InputFrame {
    axes: BTreeMap<String, f64>,
    actions: BTreeSet<String>,
}

impl Default for InputFrame {
    fn default() -> Self {
        Self {
            axes: BTreeMap::new(),
            actions: BTreeSet::new(),
        }
    }
}

impl InputFrame {
    pub fn set_axis(&mut self, name: impl Into<String>, value: f64) {
        let clamped = value.clamp(-1.0, 1.0);
        self.axes.insert(name.into(), clamped);
    }

    pub fn set_action(&mut self, name: impl Into<String>, pressed: bool) {
        let key = name.into();
        if pressed {
            self.actions.insert(key);
        } else {
            self.actions.remove(&key);
        }
    }

    pub fn axis(&self, name: &str) -> f64 {
        self.axes.get(name).copied().unwrap_or(0.0)
    }

    pub fn action_pressed(&self, name: &str) -> bool {
        self.actions.contains(name)
    }

    pub fn axes(&self) -> &BTreeMap<String, f64> {
        &self.axes
    }

    pub fn actions(&self) -> &BTreeSet<String> {
        &self.actions
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputSource {
    None,
    Scripted,
}

pub fn scripted_input_frame(frame: u32) -> InputFrame {
    scripted_input_frame_seeded(0, frame)
}

pub fn scripted_input_frame_seeded(seed: u64, frame: u32) -> InputFrame {
    let mut out = InputFrame::default();

    let move_x = if seed == 0 {
        let phase = (frame % 120) as f64 / 119.0;
        (phase * 2.0) - 1.0
    } else {
        // Stable pseudo-random input stream for replay audits.
        let mixed = splitmix64(seed ^ frame as u64);
        let normalized = (mixed as f64) / (u64::MAX as f64);
        (normalized * 2.0) - 1.0
    };
    out.set_axis("move_x", move_x.clamp(-1.0, 1.0));
    out.set_axis("move_y", 0.0);

    let pulse = if seed == 0 {
        frame % 30 == 0
    } else {
        splitmix64(seed.wrapping_add(0xA5A5_A5A5_A5A5_A5A5) ^ frame as u64) % 17 == 0
    };
    out.set_action("A", pulse);
    out.set_action("Start", frame == 0 || (seed != 0 && frame % 120 == 0));

    out
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::{scripted_input_frame, scripted_input_frame_seeded, InputFrame};

    #[test]
    fn clamps_axis_values() {
        let mut frame = InputFrame::default();
        frame.set_axis("move_x", 2.5);
        frame.set_axis("move_y", -3.2);

        assert_eq!(frame.axis("move_x"), 1.0);
        assert_eq!(frame.axis("move_y"), -1.0);
    }

    #[test]
    fn scripted_source_emits_expected_actions() {
        let f0 = scripted_input_frame(0);
        assert!(f0.action_pressed("A"));
        assert!(f0.action_pressed("Start"));

        let f1 = scripted_input_frame(1);
        assert!(!f1.action_pressed("A"));
        assert!(!f1.action_pressed("Start"));
        assert!(f1.axis("move_x") > -1.0);
    }

    #[test]
    fn seeded_scripted_input_is_reproducible() {
        let a = scripted_input_frame_seeded(1337, 9);
        let b = scripted_input_frame_seeded(1337, 9);
        let c = scripted_input_frame_seeded(1338, 9);

        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
