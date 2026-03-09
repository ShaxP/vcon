#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioBackendHealth {
    pub initialized: bool,
    pub queued_buffers: u32,
    pub underruns: u64,
    pub overruns: u64,
    pub dropped_buffers: u64,
}

#[derive(Debug, Clone)]
pub struct SimulatedAudioDevice {
    sample_rate: u32,
    buffer_frames: u32,
    max_queued_buffers: u32,
    queued_buffers: u32,
    underruns: u64,
    overruns: u64,
    dropped_buffers: u64,
    initialized: bool,
}

impl Default for SimulatedAudioDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl SimulatedAudioDevice {
    pub const BACKEND_NAME: &'static str = "simulated-device";

    pub fn new() -> Self {
        Self {
            sample_rate: 48_000,
            buffer_frames: 480,
            max_queued_buffers: 6,
            queued_buffers: 0,
            underruns: 0,
            overruns: 0,
            dropped_buffers: 0,
            initialized: true,
        }
    }

    pub fn process_frame(&mut self, dt_fixed: f64, active_voice_count: usize) {
        let frame_seconds = dt_fixed.max(1e-6);
        let produced = ((frame_seconds * self.sample_rate as f64) / (self.buffer_frames as f64))
            .ceil()
            .max(1.0) as u32;

        // Device consumes one frame slice worth of queued audio each tick.
        if self.queued_buffers < produced {
            self.underruns += (produced - self.queued_buffers) as u64;
            self.queued_buffers = 0;
        } else {
            self.queued_buffers -= produced;
        }

        // Mixer submits output buffers. With active voices we submit two slices to keep headroom.
        let submit = if active_voice_count > 0 {
            produced + 1
        } else {
            produced
        };
        self.queued_buffers = self.queued_buffers.saturating_add(submit);

        if self.queued_buffers > self.max_queued_buffers {
            let dropped = self.queued_buffers - self.max_queued_buffers;
            self.queued_buffers = self.max_queued_buffers;
            self.overruns += 1;
            self.dropped_buffers += dropped as u64;
        }
    }

    pub fn health(&self) -> AudioBackendHealth {
        AudioBackendHealth {
            initialized: self.initialized,
            queued_buffers: self.queued_buffers,
            underruns: self.underruns,
            overruns: self.overruns,
            dropped_buffers: self.dropped_buffers,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SimulatedAudioDevice;

    #[test]
    fn tracks_overrun_and_underrun_metrics() {
        let mut device = SimulatedAudioDevice::new();

        // First frame starts from empty queue, so we expect an underrun.
        device.process_frame(1.0 / 60.0, 0);
        let a = device.health();
        assert!(a.initialized);
        assert!(a.underruns > 0);

        for _ in 0..64 {
            device.process_frame(1.0 / 60.0, 8);
        }

        let b = device.health();
        assert!(b.overruns > 0);
        assert!(b.dropped_buffers > 0);
    }
}
