import vcon


class AudioSmoke(vcon.Game):
    def __init__(self):
        self.ticks = 0
        self.last_voice_id = None

    def on_boot(self):
        vcon.audio.play_music("bgm_smoke", volume=0.35, looped=True)

    def on_update(self, dt_fixed):
        self.ticks += 1

        if self.ticks % 8 == 0:
            vcon.audio.play_sfx(f"ping_{self.ticks}", volume=0.8)

        voices = vcon.audio.active_voices()
        if voices:
            self.last_voice_id = voices[-1]["voice_id"]

        if self.ticks == 24 and self.last_voice_id is not None:
            vcon.audio.stop_voice(self.last_voice_id)

    def on_render(self, alpha):
        health = vcon.audio.health()
        vcon.graphics.clear((10, 14, 20, 255))
        vcon.graphics.text("Audio Smoke", 24, 24, size=26, color=(255, 255, 255, 255))
        vcon.graphics.text(
            f"voices: {len(vcon.audio.active_voices())}",
            24,
            62,
            size=18,
            color=(220, 240, 255, 255),
        )
        vcon.graphics.text(
            f"q={health.get('queued_buffers', 0)} u={health.get('underruns', 0)} o={health.get('overruns', 0)} d={health.get('dropped_buffers', 0)}",
            24,
            90,
            size=16,
            color=(255, 225, 170, 255),
        )

    def on_shutdown(self):
        vcon.audio.stop_all()


cartridge = vcon.Cartridge(AudioSmoke())
