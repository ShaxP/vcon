"""Audio API backed by runtime mixer/playback integration."""

_play_requests = []
_stop_voice_ids = []
_stop_all = False
_runtime_active_voices = {}
_runtime_health = {
    "initialized": True,
    "queued_buffers": 0,
    "underruns": 0,
    "overruns": 0,
    "dropped_buffers": 0,
}


def _clamp_volume(volume):
    value = float(volume)
    if value < 0.0:
        return 0.0
    if value > 1.0:
        return 1.0
    return value


def play_sfx(clip_id, volume=1.0):
    key = str(clip_id)
    if not key:
        raise ValueError("clip_id must be non-empty")
    _play_requests.append({"clip_id": key, "volume": _clamp_volume(volume), "looped": False})


def play_music(clip_id, volume=1.0, looped=True):
    key = str(clip_id)
    if not key:
        raise ValueError("clip_id must be non-empty")
    _play_requests.append(
        {"clip_id": key, "volume": _clamp_volume(volume), "looped": bool(looped)}
    )


def stop_voice(voice_id):
    _stop_voice_ids.append(int(voice_id))


def stop_all():
    global _stop_all
    _stop_all = True


def active_voices():
    return [dict(_runtime_active_voices[key]) for key in sorted(_runtime_active_voices)]


def health():
    return dict(_runtime_health)


def _export_runtime_state():
    global _play_requests, _stop_voice_ids, _stop_all
    payload = {
        "play_requests": list(_play_requests),
        "stop_voice_ids": list(_stop_voice_ids),
        "stop_all": bool(_stop_all),
    }
    _play_requests = []
    _stop_voice_ids = []
    _stop_all = False
    return payload


def _set_runtime_state(active_voices, health):
    global _runtime_active_voices, _runtime_health

    next_active = {}
    for voice in active_voices:
        voice_id = int(voice["voice_id"])
        next_active[voice_id] = {
            "voice_id": voice_id,
            "clip_id": str(voice["clip_id"]),
            "volume": float(voice["volume"]),
            "looped": bool(voice["looped"]),
        }
    _runtime_active_voices = next_active

    _runtime_health = {
        "initialized": bool(health.get("initialized", True)),
        "queued_buffers": int(health.get("queued_buffers", 0)),
        "underruns": int(health.get("underruns", 0)),
        "overruns": int(health.get("overruns", 0)),
        "dropped_buffers": int(health.get("dropped_buffers", 0)),
    }
