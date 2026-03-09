use std::cell::RefCell;
use std::rc::Rc;

use anyhow::{anyhow, Context, Result};
use minifb::{Key, Window, WindowOptions};
use vcon_engine::InputFrame;

use crate::python_host::{FrameObserver, InputProvider};

struct WindowRuntimeState {
    window: Window,
    frame_buffer: Vec<u32>,
}

impl WindowRuntimeState {
    fn key_down(&self, key: Key) -> bool {
        self.window.is_key_down(key)
    }
}

pub struct WindowInputProvider {
    state: Rc<RefCell<WindowRuntimeState>>,
}

pub struct WindowFrameObserver {
    state: Rc<RefCell<WindowRuntimeState>>,
}

pub fn create_window_runtime(
    title: &str,
    width: u32,
    height: u32,
    target_fps: u32,
) -> Result<(WindowInputProvider, WindowFrameObserver)> {
    let width_usize =
        usize::try_from(width).map_err(|_| anyhow!("width out of range for platform"))?;
    let height_usize =
        usize::try_from(height).map_err(|_| anyhow!("height out of range for platform"))?;
    let mut window = Window::new(
        title,
        width_usize,
        height_usize,
        WindowOptions {
            resize: false,
            ..WindowOptions::default()
        },
    )
    .context("failed to create window")?;
    window.set_target_fps(target_fps as usize);

    let pixel_count = width_usize.saturating_mul(height_usize);
    let state = Rc::new(RefCell::new(WindowRuntimeState {
        window,
        frame_buffer: vec![0; pixel_count],
    }));

    Ok((
        WindowInputProvider {
            state: Rc::clone(&state),
        },
        WindowFrameObserver { state },
    ))
}

impl InputProvider for WindowInputProvider {
    fn next_frame(&mut self, _frame_idx: u32) -> InputFrame {
        let state = self.state.borrow();
        let left = state.key_down(Key::A) || state.key_down(Key::Left);
        let right = state.key_down(Key::D) || state.key_down(Key::Right);
        let up = state.key_down(Key::W) || state.key_down(Key::Up);
        let down = state.key_down(Key::S) || state.key_down(Key::Down);

        let mut frame = InputFrame::default();
        let move_x = if left == right {
            0.0
        } else if left {
            -1.0
        } else {
            1.0
        };
        let move_y = if up == down {
            0.0
        } else if up {
            -1.0
        } else {
            1.0
        };

        frame.set_axis("move_x", move_x);
        frame.set_axis("move_y", move_y);
        frame.set_action("DPadLeft", left);
        frame.set_action("DPadRight", right);
        frame.set_action("DPadUp", up);
        frame.set_action("DPadDown", down);
        frame.set_action("A", state.key_down(Key::Space) || state.key_down(Key::Z));
        frame.set_action("Pause", state.key_down(Key::P));
        frame.set_action(
            "Start",
            state.key_down(Key::Enter) || state.key_down(Key::R),
        );
        frame
    }
}

impl FrameObserver for WindowFrameObserver {
    fn on_frame(
        &mut self,
        _frame_idx: u32,
        frame_rgba: &[u8],
        width: u32,
        height: u32,
    ) -> Result<bool> {
        let mut state = self.state.borrow_mut();
        if !state.window.is_open() || state.window.is_key_down(Key::Escape) {
            return Ok(false);
        }

        let width_usize =
            usize::try_from(width).map_err(|_| anyhow!("width out of range for platform"))?;
        let height_usize =
            usize::try_from(height).map_err(|_| anyhow!("height out of range for platform"))?;
        let expected_pixels = width_usize.saturating_mul(height_usize);
        let expected_bytes = expected_pixels.saturating_mul(4);
        if frame_rgba.len() != expected_bytes {
            return Err(anyhow!(
                "render buffer size mismatch: expected {expected_bytes} bytes, got {}",
                frame_rgba.len()
            ));
        }

        if state.frame_buffer.len() != expected_pixels {
            state.frame_buffer.resize(expected_pixels, 0);
        }

        for (i, chunk) in frame_rgba.chunks_exact(4).enumerate() {
            let r = u32::from(chunk[0]);
            let g = u32::from(chunk[1]);
            let b = u32::from(chunk[2]);
            state.frame_buffer[i] = (r << 16) | (g << 8) | b;
        }

        let WindowRuntimeState {
            window,
            frame_buffer,
            ..
        } = &mut *state;
        window
            .update_with_buffer(frame_buffer, width_usize, height_usize)
            .context("failed to present frame to window")?;

        Ok(state.window.is_open() && !state.window.is_key_down(Key::Escape))
    }
}
