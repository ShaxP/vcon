# VCON SDK V1 Reference

Supported manifest `sdk_version`: `1`

## Lifecycle callbacks

```python
import vcon


def on_boot():
    pass


def on_update(dt_fixed: float):
    pass


def on_render(alpha: float):
    pass


def on_event(event: dict):
    # collision events use: {"type": "physics.collision", "a": str, "b": str}
    pass


def on_shutdown():
    pass
```

## Input
- `vcon.input.axis(name) -> float`
- `vcon.input.action_pressed(name) -> bool`

## Save
- `vcon.save.write(slot: str, data: dict)`
- `vcon.save.read(slot: str) -> dict | None`
- `vcon.save.list_slots() -> list[str]`

## Physics
- `vcon.physics.set_gravity(x, y)`
- `vcon.physics.upsert_body(name, x, y, vx=0.0, vy=0.0, radius=16.0, dynamic=True, restitution=0.5)`
- `vcon.physics.remove_body(name)`
- `vcon.physics.body(name) -> dict | None`
- `vcon.physics.list_bodies() -> list[dict]`

## Audio
- `vcon.audio.play_sfx(clip_id, volume=1.0)`
- `vcon.audio.play_music(clip_id, volume=1.0, looped=True)`
- `vcon.audio.stop_voice(voice_id)`
- `vcon.audio.stop_all()`
- `vcon.audio.active_voices() -> list[dict]`
- `vcon.audio.health() -> dict`
