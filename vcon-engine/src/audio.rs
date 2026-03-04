#[derive(Debug, Clone, PartialEq)]
pub struct PlayRequest {
    pub clip_id: String,
    pub volume: f32,
    pub looped: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActiveVoice {
    pub voice_id: u64,
    pub clip_id: String,
    pub volume: f32,
    pub looped: bool,
}

#[derive(Debug, Default, Clone)]
pub struct AudioMixer {
    next_voice_id: u64,
    queue: Vec<PlayRequest>,
    active: Vec<ActiveVoice>,
}

impl AudioMixer {
    pub fn queue_sfx(&mut self, clip_id: impl Into<String>, volume: f32) {
        self.queue.push(PlayRequest {
            clip_id: clip_id.into(),
            volume: volume.clamp(0.0, 1.0),
            looped: false,
        });
    }

    pub fn queue_music(&mut self, clip_id: impl Into<String>, volume: f32, looped: bool) {
        self.queue.push(PlayRequest {
            clip_id: clip_id.into(),
            volume: volume.clamp(0.0, 1.0),
            looped,
        });
    }

    pub fn flush_queue(&mut self) -> &[ActiveVoice] {
        for req in self.queue.drain(..) {
            self.next_voice_id += 1;
            self.active.push(ActiveVoice {
                voice_id: self.next_voice_id,
                clip_id: req.clip_id,
                volume: req.volume,
                looped: req.looped,
            });
        }

        &self.active
    }

    pub fn active_voices(&self) -> &[ActiveVoice] {
        &self.active
    }

    pub fn stop_voice(&mut self, voice_id: u64) {
        self.active.retain(|v| v.voice_id != voice_id);
    }

    pub fn stop_all(&mut self) {
        self.active.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::AudioMixer;

    #[test]
    fn queues_and_activates_unlimited_voices() {
        let mut mixer = AudioMixer::default();

        for i in 0..128 {
            mixer.queue_sfx(format!("sfx_{i}"), 0.8);
        }
        let active = mixer.flush_queue();
        assert_eq!(active.len(), 128);
    }

    #[test]
    fn supports_music_loop_and_stop() {
        let mut mixer = AudioMixer::default();
        mixer.queue_music("bgm_title", 0.5, true);
        let first = mixer.flush_queue()[0].clone();
        assert!(first.looped);

        mixer.stop_voice(first.voice_id);
        assert!(mixer.active_voices().is_empty());
    }
}
