# VCON SDK V2 Reference

Supported manifest `sdk_version`: `2`

## Cartridge contract

```python
import vcon


class MyGame(vcon.Game):
    def on_boot(self) -> None:
        pass

    def on_update(self, dt_fixed: float) -> None:
        pass

    def on_render(self, alpha: float) -> None:
        pass

    def on_event(self, event: dict) -> None:
        # collision events use: {"type": "physics.collision", "a": str, "b": str}
        pass

    def on_shutdown(self) -> None:
        pass

cartridge = vcon.Cartridge(MyGame())
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

## FSM
- `vcon.fsm.State(context, machine)`
- `vcon.fsm.StateMachine(context)`
- `StateMachine.change_state(next_state)`
- `StateMachine.update(dt_seconds)`
- `StateMachine.render(alpha)`
- `StateMachine.on_event(event)`
