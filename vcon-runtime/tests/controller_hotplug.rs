use std::fs;
use std::path::PathBuf;

mod python_host {
    use vcon_engine::InputFrame;

    pub trait InputProvider {
        fn next_frame(&mut self, frame_idx: u32) -> InputFrame;
    }
}

#[path = "../src/gamepad.rs"]
mod gamepad;

use gamepad::{ControllerBackendKind, GamepadInputProvider};
use python_host::InputProvider;

fn unique_state_file() -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    path.push(format!("vcon-controller-hotplug-{nanos}.txt"));
    path
}

#[test]
fn covers_connect_disconnect_reconnect_and_profile_remap_paths() {
    let state_file = unique_state_file();
    fs::write(
        &state_file,
        "connected=true\nprofile=desktop\nmove_x=0.09\nA=true\nStart=true\n",
    )
    .expect("should write initial state");

    let mut provider =
        GamepadInputProvider::with_backend(ControllerBackendKind::File, state_file.clone());

    let f0 = provider.next_frame(0);
    assert!(!f0.action_pressed("ControllerConnectedState"));

    let f1 = provider.next_frame(1);
    assert!(f1.action_pressed("ControllerConnected"));
    assert!(f1.action_pressed("ControllerConnectedState"));
    assert_eq!(f1.axis("move_x"), 0.0, "desktop deadzone should zero 0.09");

    fs::write(&state_file, "connected=false\n").expect("should write disconnect state");

    let f2 = provider.next_frame(2);
    assert!(f2.action_pressed("ControllerConnectedState"));

    let f3 = provider.next_frame(3);
    assert!(f3.action_pressed("ControllerDisconnected"));
    assert!(!f3.action_pressed("ControllerConnectedState"));
    assert_eq!(f3.axis("move_x"), 0.0);
    assert!(!f3.action_pressed("A"));

    fs::write(
        &state_file,
        "connected=true\nprofile=steamdeck\nmove_x=0.09\nA=true\n",
    )
    .expect("should write reconnect state");

    let f4 = provider.next_frame(4);
    assert!(!f4.action_pressed("ControllerConnectedState"));

    let f5 = provider.next_frame(5);
    assert!(f5.action_pressed("ControllerReconnected"));
    assert!(f5.action_pressed("ControllerConnectedState"));
    assert_eq!(f5.axis("move_x"), 0.09, "steam deck profile should retain 0.09");

    let _ = fs::remove_file(state_file);
}
