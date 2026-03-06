import vcon

_ticks = 0
_last_voice_id = None


def on_boot():
    vcon.audio.play_music("bgm_smoke", volume=0.35, looped=True)


def on_update(dt_fixed):
    global _ticks, _last_voice_id
    _ticks += 1

    if _ticks % 8 == 0:
        vcon.audio.play_sfx(f"ping_{_ticks}", volume=0.8)

    voices = vcon.audio.active_voices()
    if voices:
        _last_voice_id = voices[-1]["voice_id"]

    if _ticks == 24 and _last_voice_id is not None:
        vcon.audio.stop_voice(_last_voice_id)


def on_render(alpha):
    h = vcon.audio.health()
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
        f"q={h.get('queued_buffers', 0)} u={h.get('underruns', 0)} o={h.get('overruns', 0)} d={h.get('dropped_buffers', 0)}",
        24,
        90,
        size=16,
        color=(255, 225, 170, 255),
    )


def on_event(event):
    return None


def on_shutdown():
    vcon.audio.stop_all()
